use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

/// FFT window size. 2048 gives ~23 Hz resolution at 48 kHz, enough to separate
/// bass notes that a 1024-point transform smears together.
const FFT_SIZE: usize = 2048;

/// Number of log-spaced visualizer bands. Must stay in sync with the frontend
/// spectrum meter, which renders this many bars.
pub const NUM_BANDS: usize = 24;

/// Musical range the bands cover.
///
/// The floor is 50 Hz rather than 30 Hz because almost nothing in consumer audio
/// has real energy below that, and speakers roll off there. Bands mapped under
/// 50 Hz therefore sat permanently dark - which is what made the bottom rows of
/// the display never light. The ceiling is 10 kHz because above that is mostly
/// noise and cymbal wash.
const F_MIN: f32 = 50.0;
const F_MAX: f32 = 10_000.0;

/// Pink noise falls about 3 dB per octave. Compensating for that is what makes
/// the treble bands respond as strongly as the bass ones.
const TILT_DB_PER_OCTAVE: f32 = 3.0;

/// Dynamic range mapped onto the LED height, in dB below the tracked peak.
const DYNAMIC_RANGE_DB: f32 = 42.0;

/// Headroom below the tracked reference. Without this the loudest band is always
/// normalized to exactly 100%, so quiet passages look as loud as loud ones and the
/// display feels permanently maxed out. This leaves peaks around 76%, so 1x is a
/// comfortable default and the knob still has useful range on both sides.
const HEADROOM_DB: f32 = 10.0;

/// How fast the adaptive gain reference falls, in dB per second.
const AGC_RELEASE_DB_PER_SEC: f32 = 8.0;

/// Below this magnitude the input is just noise floor, so we output true zero.
const GATE_MAG: f32 = 0.0006;

/// Compute per-band FFT bin ranges, log-spaced so every band covers an equal
/// number of octaves. Grouping these by index is therefore a sensible musical
/// split (bass / mid / treble), unlike the old linear-frequency table.
struct BandTable {
    edges: Vec<(usize, usize)>,
    tilt_db: Vec<f32>,
}

impl BandTable {
    fn new(sample_rate: f32) -> Self {
        let bin_hz = sample_rate / FFT_SIZE as f32;
        let nyquist_bin = FFT_SIZE / 2;
        let ratio = (F_MAX / F_MIN).powf(1.0 / NUM_BANDS as f32);
        let octaves_per_band = (F_MAX / F_MIN).log2() / NUM_BANDS as f32;

        let mut edges = Vec::with_capacity(NUM_BANDS);
        let mut tilt_db = Vec::with_capacity(NUM_BANDS);
        let mut f = F_MIN;

        for i in 0..NUM_BANDS {
            let f_next = f * ratio;
            let start = ((f / bin_hz).round() as usize).max(1);
            let end = ((f_next / bin_hz).round() as usize)
                .max(start + 1)
                .min(nyquist_bin);
            edges.push((start, end));
            tilt_db.push(i as f32 * octaves_per_band * TILT_DB_PER_OCTAVE);
            f = f_next;
        }

        Self { edges, tilt_db }
    }
}

#[derive(Clone)]
pub struct AudioVisualizer {
    pub energy: Arc<Mutex<Vec<f32>>>,
    pub sensitivity: Arc<AtomicU32>, // Percentage: 100 = 1.0x, 200 = 2.0x
    running: Arc<AtomicBool>,
    handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

unsafe impl Send for AudioVisualizer {}
unsafe impl Sync for AudioVisualizer {}

impl AudioVisualizer {
    pub fn new() -> Self {
        Self {
            energy: Arc::new(Mutex::new(vec![0.0f32; NUM_BANDS])),
            sensitivity: Arc::new(AtomicU32::new(100)),
            running: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Start capture if not already running. Joins any previous thread first, so
    /// a quick stop/start pair can't race and leave capture permanently off.
    pub fn start(&self) {
        let mut slot = self.handle.lock().unwrap();
        if let Some(prev) = slot.take() {
            self.running.store(false, Ordering::SeqCst);
            let _ = prev.join();
        }
        self.running.store(true, Ordering::SeqCst);

        let energy = Arc::clone(&self.energy);
        let sensitivity = Arc::clone(&self.sensitivity);
        let running = Arc::clone(&self.running);

        *slot = Some(std::thread::spawn(move || {
            capture_loop(energy, sensitivity, running);
        }));
    }

    /// Signal capture to stop and wait for the thread to exit.
    pub fn stop(&self) {
        let mut slot = self.handle.lock().unwrap();
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = slot.take() {
            let _ = handle.join();
        }
        // Clear stale bands so a mode switch doesn't briefly show frozen values.
        if let Ok(mut eng) = self.energy.lock() {
            for v in eng.iter_mut() {
                *v = 0.0;
            }
        }
    }

    /// Start or stop capture depending on whether the mode needs audio.
    pub fn set_needed(&self, needed: bool) {
        let mut slot = self.handle.lock().unwrap();

        // Drop a finished thread (e.g. capture failed to initialise) so it can retry.
        if slot.as_ref().is_some_and(|h| h.is_finished()) {
            let _ = slot.take();
        }

        let running = slot.is_some();
        if needed == running {
            return;
        }
        drop(slot);

        if needed {
            self.start();
        } else {
            self.stop();
        }
    }

    pub fn is_active(&self) -> bool {
        self.handle
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|h| !h.is_finished())
    }
}

/// Capture + FFT loop. Owns the cpal stream for its lifetime.
/// Only runs while the app is in a mode that reacts to audio, so idle modes
/// don't hold a WASAPI loopback stream open (which also keeps the audio
/// endpoint awake) or burn CPU on an unused FFT.
fn capture_loop(
    energy_clone: Arc<Mutex<Vec<f32>>>,
    sensitivity_clone: Arc<AtomicU32>,
    running_clone: Arc<AtomicBool>,
) {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            log::warn!("No default audio output device found for loopback capture");
            return;
        }
    };

    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to query default output config: {}", e);
            return;
        }
    };

    let channels = config.channels() as usize;
    // Needed to turn FFT bins into frequencies. Captured before `config` is
    // consumed by `config.into()` in the stream builders below.
    let sample_rate = config.sample_rate().0 as f32;
    let buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(2048)));
    let buffer_inner = Arc::clone(&buffer);
    let buffer_i16 = Arc::clone(&buffer);
    let buffer_u16 = Arc::clone(&buffer);

    let err_fn = |err| log::error!("Audio loopback stream error: {}", err);

    let stream_result = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                let mut buf = buffer_inner.lock().unwrap();
                for frame in data.chunks(channels) {
                    let mono = frame.iter().sum::<f32>() / channels as f32;
                    buf.push(mono);
                    // Amortized trim: draining per sample meant a memmove
                    // on every one of ~48k samples/sec.
                    if buf.len() > 4096 {
                        let excess = buf.len() - 2048;
                        buf.drain(0..excess);
                    }
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| {
                let mut buf = buffer_i16.lock().unwrap();
                for frame in data.chunks(channels) {
                    let mono =
                        frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32;
                    buf.push(mono);
                    if buf.len() > 4096 {
                        let excess = buf.len() - 2048;
                        buf.drain(0..excess);
                    }
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _: &_| {
                let mut buf = buffer_u16.lock().unwrap();
                for frame in data.chunks(channels) {
                    let mono = frame
                        .iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .sum::<f32>()
                        / channels as f32;
                    buf.push(mono);
                    if buf.len() > 4096 {
                        let excess = buf.len() - 2048;
                        buf.drain(0..excess);
                    }
                }
            },
            err_fn,
            None,
        ),
        other => {
            log::warn!("Unsupported audio sample format: {:?}", other);
            return;
        }
    };

    let _stream = match stream_result {
        Ok(s) => {
            if let Err(e) = s.play() {
                log::error!("Error starting audio stream: {}", e);
                return;
            }
            s
        }
        Err(e) => {
            log::warn!("Could not build loopback input stream: {}", e);
            return;
        }
    };

    // FFT over a Hann-windowed 2048-sample frame.
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let bands_table = BandTable::new(sample_rate);

    // Pre-allocated scratch buffers so the loop allocates nothing per frame.
    let mut samples: Vec<f32> = vec![0.0; FFT_SIZE];
    let mut input: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); FFT_SIZE];
    // Precompute the Hann window; it never changes.
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()))
        .collect();

    let mut raw_db = vec![-120.0f32; NUM_BANDS];
    // Adaptive reference level. Brightness is measured relative to this, so the
    // effect works at any volume instead of assuming a fixed dB range.
    let mut gain_ref_db = -40.0f32;
    let mut frame_started = Instant::now();

    // Bin magnitude that corresponds to full scale, used to normalize to 0 dBFS.
    let norm_factor = FFT_SIZE as f32 / 4.0;

    while running_clone.load(Ordering::Relaxed) {
        // ~30 FPS, matching the LED engine.
        std::thread::sleep(std::time::Duration::from_millis(33));

        {
            let buf = buffer.lock().unwrap();
            if buf.len() < FFT_SIZE {
                continue;
            }
            samples.copy_from_slice(&buf[buf.len() - FFT_SIZE..]);
        }

        let dt = frame_started.elapsed().as_secs_f32().clamp(0.001, 0.1);
        frame_started = Instant::now();

        // Apply the window and run the transform.
        let mut frame_peak = 0.0f32;
        for i in 0..FFT_SIZE {
            let v = samples[i] * window[i];
            if v.abs() > frame_peak {
                frame_peak = v.abs();
            }
            input[i] = Complex::new(v, 0.0);
        }
        fft.process(&mut input);

        // Average magnitude per log-spaced band, converted to dBFS.
        // NOTE: sensitivity is deliberately NOT applied here. Adding a constant dB
        // offset to every band cancelled out against the AGC reference below, which
        // made the knob almost a no-op. It is applied as an output gain instead.
        let sens = (sensitivity_clone.load(Ordering::Relaxed) as f32 / 100.0).clamp(0.1, 5.0);

        for (idx, &(start, end)) in bands_table.edges.iter().enumerate() {
            let mut sum = 0.0f32;
            for bin in start..end.min(input.len() / 2) {
                sum += input[bin].norm();
            }
            let count = (end.saturating_sub(start)).max(1) as f32;
            let mag = sum / count;

            // Normalize to full scale, then to dB, then apply the pink-noise tilt.
            let db = 20.0 * (mag / norm_factor).max(1e-7).log10();
            raw_db[idx] = db + bands_table.tilt_db[idx];
        }

        // Adaptive gain: track the loudest band, rise quickly, fall slowly.
        // Everything is displayed relative to this, which keeps the display
        // lively at low volume and prevents clipping at high volume.
        let loudest = raw_db.iter().copied().fold(-120.0f32, f32::max);
        if loudest > gain_ref_db {
            gain_ref_db = loudest;
        } else {
            gain_ref_db = (gain_ref_db - AGC_RELEASE_DB_PER_SEC * dt).max(loudest);
        }
        // Reference includes headroom so peaks land below full scale.
        let floor_db = gain_ref_db - DYNAMIC_RANGE_DB + HEADROOM_DB;

        // True silence must give exactly zero, otherwise the LEDs glow faintly.
        let is_silent = frame_peak < GATE_MAG;

        if let Ok(mut eng) = energy_clone.lock() {
            for i in 0..NUM_BANDS {
                let target = if is_silent {
                    0.0
                } else {
                    // Sensitivity is a plain output gain, so the knob behaves
                    // predictably: 1x is the tuned default, higher reacts more.
                    let normalized =
                        ((raw_db[i] - floor_db) / DYNAMIC_RANGE_DB).clamp(0.0, 1.0);
                    (normalized * sens).clamp(0.0, 1.0)
                };
                if target >= eng[i] {
                    // Instant attack - lights on the same frame the sound arrives.
                    eng[i] = target;
                } else {
                    // Fast, time-based release so bars fall smoothly and reach 0.
                    eng[i] = target + (eng[i] - target) * 0.72f32.powf(dt / 0.033);
                }
            }
        }
    }
}

impl AudioVisualizer {
    pub fn get_bands(&self) -> Vec<f32> {
        if let Ok(b) = self.energy.lock() {
            b.clone()
        } else {
            vec![0.0f32; NUM_BANDS]
        }
    }

    pub fn set_sensitivity(&self, sens: f32) {
        let pct = (sens.clamp(0.1, 5.0) * 100.0).round() as u32;
        self.sensitivity.store(pct, Ordering::Relaxed);
    }

    pub fn get_sensitivity(&self) -> f32 {
        self.sensitivity.load(Ordering::Relaxed) as f32 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_loopback() {
        let viz = AudioVisualizer::new();
        viz.start();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let bands = viz.get_bands();
        println!("Test audio bands output ({} bands): {:?}", NUM_BANDS, bands);
        assert_eq!(bands.len(), NUM_BANDS);
        viz.stop();
    }
}
