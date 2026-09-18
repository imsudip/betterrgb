use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
pub struct AudioVisualizer {
    pub energy: Arc<Mutex<Vec<f32>>>,
    pub sensitivity: Arc<AtomicU32>, // Percentage: 100 = 1.0x, 200 = 2.0x
    #[allow(dead_code)]
    running: Arc<AtomicBool>,
}

unsafe impl Send for AudioVisualizer {}
unsafe impl Sync for AudioVisualizer {}

impl AudioVisualizer {
    pub fn new() -> Self {
        let energy = Arc::new(Mutex::new(vec![0.0f32; 32]));
        let sensitivity = Arc::new(AtomicU32::new(100)); // Default 100% (1.0x)
        let running = Arc::new(AtomicBool::new(true));

        let energy_clone = Arc::clone(&energy);
        let sensitivity_clone = Arc::clone(&sensitivity);
        let running_clone = Arc::clone(&running);

        std::thread::spawn(move || {
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
                            let len = buf.len();
                            if len > 2048 {
                                buf.drain(0..len - 2048);
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
                            let mono = frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32;
                            buf.push(mono);
                            let len = buf.len();
                            if len > 2048 {
                                buf.drain(0..len - 2048);
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
                            let mono = frame.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).sum::<f32>() / channels as f32;
                            buf.push(mono);
                            let len = buf.len();
                            if len > 2048 {
                                buf.drain(0..len - 2048);
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

            // High-resolution 1024-point FFT planner
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(1024);

            while running_clone.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(20)); // ~50 FPS responsive updates

                let samples = {
                    let buf = buffer.lock().unwrap();
                    if buf.len() < 1024 {
                        continue;
                    }
                    buf[buf.len() - 1024..].to_vec()
                };

                let mut input: Vec<Complex<f32>> = samples
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| {
                        let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / 1024.0).cos());
                        Complex::new(s * window, 0.0)
                    })
                    .collect();

                fft.process(&mut input);

                let mut bands = vec![0.0f32; 32];
                let bin_ranges = [
                    (1, 2),     // 0: ~25-47 Hz (Sub-bass ultra low)
                    (2, 3),     // 1: ~47-94 Hz (Sub-bass kick drum)
                    (3, 4),     // 2: ~94-141 Hz (Bass low)
                    (4, 5),     // 3: ~141-188 Hz (Bass mid)
                    (5, 6),     // 4: ~188-234 Hz (Bass punch)
                    (6, 7),     // 5: ~234-281 Hz (Bass upper)
                    (7, 9),     // 6: ~281-375 Hz (Low-mid warmth)
                    (9, 11),    // 7: ~375-469 Hz (Low-mid body)
                    (11, 13),   // 8: ~469-563 Hz (Mid low)
                    (13, 16),   // 9: ~563-703 Hz (Mid range)
                    (16, 20),   // 10: ~703-890 Hz (Mid presence)
                    (20, 24),   // 11: ~890-1078 Hz (Vocal warmth)
                    (24, 29),   // 12: ~1.08-1.31 kHz (Vocal clarity)
                    (29, 35),   // 13: ~1.31-1.59 kHz
                    (35, 42),   // 14: ~1.59-1.92 kHz
                    (42, 50),   // 15: ~1.92-2.30 kHz
                    (50, 60),   // 16: ~2.30-2.76 kHz
                    (60, 72),   // 17: ~2.76-3.32 kHz
                    (72, 86),   // 18: ~3.32-3.98 kHz
                    (86, 103),  // 19: ~3.98-4.78 kHz
                    (103, 123), // 20: ~4.78-5.71 kHz
                    (123, 147), // 21: ~5.71-6.84 kHz
                    (147, 175), // 22: ~6.84-8.15 kHz
                    (175, 208), // 23: ~8.15-9.69 kHz
                    (208, 246), // 24: ~9.69-11.48 kHz
                    (246, 290), // 25: ~11.48-13.59 kHz
                    (290, 340), // 26: ~13.59-15.94 kHz
                    (340, 395), // 27: ~15.94-18.52 kHz
                    (395, 440), // 28: ~18.52-20.63 kHz
                    (440, 470), // 29: ~20.63-22.03 kHz
                    (470, 495), // 30: ~22.03-23.20 kHz
                    (495, 511), // 31: ~23.20-23.95 kHz
                ];

                // Track overall signal energy to gate background noise
                let mut max_raw = 0.0f32;
                let sens = (sensitivity_clone.load(Ordering::Relaxed) as f32 / 100.0).clamp(0.1, 5.0);

                for (idx, &(start, end)) in bin_ranges.iter().enumerate() {
                    let mut sum = 0.0f32;
                    let end_clamp = end.min(input.len() / 2);
                    for i in start..end_clamp {
                        sum += input[i].norm();
                    }
                    let count = (end_clamp.saturating_sub(start)).max(1) as f32;
                    // Normalize by FFT size (512 is half of 1024) and scale by sensitivity knob
                    let avg = ((sum / count) / 512.0) * sens;
                    if avg > max_raw {
                        max_raw = avg;
                    }

                    // Logarithmic Decibel calculation (dynamic range from -58 dB to -6 dB)
                    let db = 20.0 * (avg.max(1e-5)).log10();
                    // Frequency tilt (+0.45 dB per band to balance pink-noise roll-off in music)
                    let tilt_db = idx as f32 * 0.45;
                    let compensated_db = db + tilt_db;

                    // Map [-56.0 dB, -8.0 dB] to [0.0, 1.0]
                    let min_db = -56.0f32;
                    let max_db = -8.0f32;
                    let normalized = ((compensated_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);

                    // Power curve (gamma) for punchy, dynamic visual movement
                    bands[idx] = normalized.powf(1.4);
                }

                // Noise gate: if total signal is negligible silence (< -65 dB), drop to 0
                let is_silent = max_raw < 0.0008;

                if let Ok(mut eng) = energy_clone.lock() {
                    for i in 0..32 {
                        let target = if is_silent { 0.0 } else { bands[i] };
                        if target > eng[i] {
                            // Fast transient attack
                            eng[i] = eng[i] * 0.35 + target * 0.65;
                        } else {
                            // Smooth visual decay that cleanly returns to 0
                            eng[i] = (eng[i] * 0.85 - 0.005).max(0.0);
                        }
                    }
                }
            }
        });

        Self {
            energy,
            sensitivity,
            running,
        }
    }

    pub fn get_bands(&self) -> Vec<f32> {
        if let Ok(b) = self.energy.lock() {
            b.clone()
        } else {
            vec![0.0f32; 32]
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
        std::thread::sleep(std::time::Duration::from_millis(500));
        let bands = viz.get_bands();
        println!("Test audio bands output (32 bands): {:?}", bands);
        assert_eq!(bands.len(), 32);
    }
}
