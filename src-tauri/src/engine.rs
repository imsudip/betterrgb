use crate::audio::AudioVisualizer;
use crate::font;
use crate::openrgb::{DeviceInfo, OpenRgbClient, RgbColor, ORGB_DEFAULT_PORT};
use crate::sensors::{ActiveWindowTracker, HardwareSensors};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tauri::Emitter;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Smart,      // Auto-reacts: coding -> dim cool, gaming -> thermal, idle -> breathing
    Thermal,    // Direct hardware temperature to color mapping
    Visualizer, // React to audio beats/frequencies
    Coding,     // Force coding mode (dim cool focus cyan/blue)
    Gaming,     // Force gaming aggressive profile
    Static,     // Solid color
    Breathing,  // Smooth pulsing
    Rainbow,    // Moving spectrum
    AppSync,    // Adaptive foreground app logo/icon glow
    Text,       // Scrolling marquee message across the panel
    Snake,      // Self-playing snake demo
    Tetris,     // Self-playing tetris demo
    Off,        // All LEDs zero
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatePayload {
    pub connected: bool,
    pub active_mode: AppMode,
    pub brightness: f32, // 0.0 - 1.0
    pub static_color: RgbColor,
    pub current_color: RgbColor,
    pub cpu_usage: f32,
    pub cpu_temp: Option<f32>,
    pub active_app: String,
    pub is_coding_detected: bool,
    pub is_gaming_detected: bool,
    pub devices: Vec<DeviceInfo>,
    pub led_count: u32,
    pub audio_bands: Vec<f32>,
    pub audio_sensitivity: f32,
    pub app_color: RgbColor,
    pub debug: DebugPattern,
    pub debug_index: u32,
    pub debug_paused: bool,
    pub layout: PanelLayout,
    pub viz_style: VizStyle,
    pub marquee_text: String,
    /// Play-speed multiplier for the game demos, so the UI control can round-trip.
    pub game_speed: f32,
}

/// Diagnostic test patterns used by the Debug page to characterise the hardware.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugPattern {
    /// Normal operation.
    Off,
    /// Every LED white - proves which LEDs physically respond.
    AllWhite,
    /// Repeating 10 red / 10 green / 10 blue / 10 white - count the runs.
    Count,
    /// A pulse travelling one LED at a time, with a numbered position. Used to
    /// map a logical index onto a physical LED.
    Snake,
    /// Only the LED at `debug_index` - the key test for daisy-chain vs parallel.
    Single,
    /// Every 10th LED lit - a ruler for counting without losing your place.
    Tens,
    /// Front half red, back half blue - reveals mirroring between channels.
    Halves,
}

/// Visual style for the audio visualizer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VizStyle {
    /// One vertical bar per lane: bass / mid / treble rising from the bottom.
    Columns,
    /// One row per frequency band, each row lit fully across all lanes.
    Rows,
    /// Brightness blooming outward from the centre row.
    Bloom,
}

/// Physical arrangement of the LEDs as a grid.
///
/// Confirmed for this fan: 3 lanes (columns) of 13 LEDs (rows), index 0 unwired,
/// and every lane runs bottom to top.
///
/// The strip is a GRID, not a line. Index order advances along a lane first, so
/// anything computed along the linear index appears to restart at the bottom
/// every 13 LEDs. Effects are therefore computed per (row, column) and a row
/// spans all lanes, which keeps them visually continuous across the fan.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelLayout {
    /// Number of addressable lanes (columns).
    pub lanes: u32,
    /// LEDs per lane (rows).
    pub leds_per_lane: u32,
    /// Unwired slots at the start of the strip. Index 0 is dead on this device.
    pub first_index: u32,
    /// True if odd lanes run top-to-bottom (serpentine). False here: all lanes
    /// run bottom-to-top.
    pub serpentine: bool,
    /// Place bass at the top of each lane instead of the bottom. Lets the
    /// spectrum be flipped to match how the fan is oriented in the case.
    pub bass_at_top: bool,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            lanes: 3,
            leds_per_lane: 13,
            first_index: 1,
            serpentine: false,
            bass_at_top: false,
        }
    }
}

impl PanelLayout {
    /// Total addressable slots the wiring covers, including unwired lead-in.
    pub fn total_slots(&self) -> u32 {
        self.first_index + self.lanes * self.leds_per_lane
    }

    /// Convert a grid coordinate to a raw strip index.
    /// `row` 0 is the bottom; `col` 0 is the first lane.
    pub fn index_of(&self, row: u32, col: u32) -> Option<u32> {
        if row >= self.leds_per_lane || col >= self.lanes {
            return None;
        }
        // Odd lanes are reversed when the strip is wired serpentine.
        let r = if self.serpentine && col % 2 == 1 {
            self.leds_per_lane - 1 - row
        } else {
            row
        };
        Some(self.first_index + col * self.leds_per_lane + r)
    }
}

/// Render a grid effect into a strip frame.
///
/// `f(row, col)` returns the colour for one LED, with `row` 0 at the bottom and
/// `col` 0 as the first lane. Everything outside the wired area stays dark, so
/// the unused lead-in index is handled automatically.
pub fn render_grid<F>(layout: &PanelLayout, led_count: u32, f: F) -> Vec<RgbColor>
where
    F: Fn(u32, u32) -> RgbColor,
{
    let mut out = vec![RgbColor::new(0, 0, 0); led_count as usize];
    for col in 0..layout.lanes {
        for row in 0..layout.leds_per_lane {
            if let Some(idx) = layout.index_of(row, col) {
                if let Some(slot) = out.get_mut(idx as usize) {
                    *slot = f(row, col);
                }
            }
        }
    }
    out
}

pub struct EngineConfig {
    pub mode: AppMode,
    pub brightness: f32,
    pub static_color: RgbColor,
    pub led_count: u32,
    pub speed: f32,
    /// Active diagnostic pattern. `Off` restores normal lighting.
    pub debug: DebugPattern,
    /// Focus LED. For `Single` it is the lit LED; for `Snake` it is the head
    /// position, which auto-advances while `debug_paused` is false.
    pub debug_index: u32,
    /// Freezes the snake head so a specific index can be inspected.
    pub debug_paused: bool,
    /// Set when the user explicitly picks an index, so the snake jumps there.
    /// Distinct from `debug_index`, which the engine also updates as the head
    /// travels - without this the engine cannot tell its own movement apart
    /// from a user request.
    pub debug_jump: bool,
    /// Physical LED arrangement. Effects are rendered per (row, column).
    pub layout: PanelLayout,
    /// Which layout the audio visualizer uses.
    pub viz_style: VizStyle,
    /// Message shown by `AppMode::Text`.
    pub marquee_text: String,
    /// Scroll rate for the marquee, in pixels per second.
    pub marquee_speed: f32,
    /// Play-speed multiplier for the Snake and Tetris demos. 1.0 is the tuned
    /// default; higher is faster.
    pub game_speed: f32,
}

pub struct RgbEngine {
    pub config: Arc<Mutex<EngineConfig>>,
    pub state_payload: Arc<Mutex<AppStatePayload>>,
    running: Arc<AtomicBool>,
}

impl RgbEngine {
    pub fn new() -> Self {
        let initial_config = EngineConfig {
            mode: AppMode::Smart,
            brightness: 0.8,
            static_color: RgbColor::new(0, 210, 255), // Vibrant cyber cyan
            led_count: 40, // 3 lanes x 13 + 1 unused slot (index 0 is unwired)
            speed: 1.0,
            debug: DebugPattern::Off,
            debug_index: 0,
            debug_paused: false,
            debug_jump: false,
            layout: PanelLayout::default(),
            viz_style: VizStyle::Columns,
            marquee_text: "BETTERRGB".into(),
            marquee_speed: 14.0,
            game_speed: 1.0,
        };

        let initial_payload = AppStatePayload {
            connected: false,
            active_mode: AppMode::Smart,
            brightness: 0.8,
            static_color: RgbColor::new(0, 210, 255),
            current_color: RgbColor::new(0, 210, 255),
            cpu_usage: 0.0,
            cpu_temp: None,
            active_app: "Desktop".into(),
            is_coding_detected: false,
            is_gaming_detected: false,
            devices: Vec::new(),
            led_count: 40,
            audio_bands: vec![0.0; 32],
            audio_sensitivity: 1.0,
            app_color: RgbColor::new(0, 210, 255),
            debug: DebugPattern::Off,
            debug_index: 0,
            debug_paused: false,
            layout: PanelLayout::default(),
            viz_style: VizStyle::Columns,
            marquee_text: "BETTERRGB".into(),
            game_speed: 1.0,
        };

        Self {
            config: Arc::new(Mutex::new(initial_config)),
            state_payload: Arc::new(Mutex::new(initial_payload)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self, audio: Arc<AudioVisualizer>, app: tauri::AppHandle) {
        let config_clone = Arc::clone(&self.config);
        let payload_clone = Arc::clone(&self.state_payload);
        let running_clone = Arc::clone(&self.running);
        let audio_clone = Arc::clone(&audio);

        std::thread::spawn(move || {
            let mut sensors = HardwareSensors::new();
            let mut window_tracker = ActiveWindowTracker::new();
            let mut openrgb_client: Option<OpenRgbClient> = None;
            let mut devices_cache: Vec<DeviceInfo> = Vec::new();
            let mut tick: f32 = 0.0;
            let mut frame_count: u64 = 0;
            let mut last_requested_led_count: u32 = 0;

            // Cached sensor/window readings, refreshed at 1 Hz instead of every frame.
            let mut cpu_usage = 0.0f32;
            let mut cpu_temp: Option<f32> = None;
            let mut app_color = RgbColor::new(0, 210, 255);
            let mut window_info = window_tracker.get_active_window_info();
            let mut last_sensor_poll = Instant::now() - Duration::from_secs(1);

            // Per-device last-sent frame, so identical frames aren't re-sent over TCP.
            let mut last_sent: Vec<(u32, Vec<RgbColor>)> = Vec::new();
            let mut scratch: Vec<RgbColor> = Vec::with_capacity(240);
            let mut frame_started = Instant::now();

            // Snake head lives outside the loop. It cannot be re-derived from the
            // config each frame, because the config only changes when the user
            // edits the index - the head would reset to that value every frame
            // and never travel.
            let mut snake_pos: f32 = 0.0;
            // Marquee scroll offset in pixels. Advances with real time so the
            // speed is independent of the tick rate.
            let mut marquee_scroll: f32 = 0.0;
            // Self-playing games. Each holds its own board and advances from real
            // elapsed time, so they stay in step at any tick rate. Sized from the
            // default layout here; `sync_shape` resizes them if it changes.
            // NOTE: `Games::new` takes (lanes, leds_per_lane). Swapping those
            // silently clamps the board to 3 rows and leaves the panel dark.
            let default_layout = PanelLayout::default();
            let mut games = crate::games::Games::new(
                default_layout.lanes.max(1) as usize,
                default_layout.leds_per_lane.max(1) as usize,
            );
            // Peak-hold levels for the visualizer, one per lane. Indexed by column
            // so the marker on each bar is independent.
            let mut peaks: Vec<f32> = vec![0.0; 8];

            while running_clone.load(Ordering::Relaxed) {
                let (mode, brightness, static_color, led_count, speed, debug, mut debug_index, debug_paused, debug_jump, layout, viz_style, marquee_text, marquee_speed, game_speed) = {
                    let cfg = config_clone.lock().unwrap();
                    (
                        cfg.mode.clone(),
                        cfg.brightness,
                        cfg.static_color.clone(),
                        cfg.led_count,
                        cfg.speed,
                        cfg.debug.clone(),
                        cfg.debug_index,
                        cfg.debug_paused,
                        cfg.debug_jump,
                        cfg.layout,
                        cfg.viz_style,
                        cfg.marquee_text.clone(),
                        cfg.marquee_speed,
                        cfg.game_speed,
                    )
                };

                // Tick rate scales with how much motion the mode actually needs.
                // Static/Off are effectively free; only animated modes need a high
                // frame rate. Audio-reactive modes stay at 30 FPS (the rate the
                // engine always used), so LED smoothness is unchanged. Diagnostics
                // and the games always run at full rate so motion is smooth
                // regardless of the selected lighting mode.
                let frame_ms: u64 = if debug != DebugPattern::Off {
                    33
                } else {
                    match mode {
                        AppMode::Off => 500,
                        AppMode::Static | AppMode::Coding => 250,
                        AppMode::Breathing | AppMode::Thermal | AppMode::AppSync => 100,
                        _ => 33,
                    }
                };
                std::thread::sleep(Duration::from_millis(frame_ms));

                // Advance animation time by real elapsed seconds, scaled so the
                // animation speed matches the original fixed-step behaviour
                // (0.05 units per 33ms frame) at every tick rate.
                let elapsed = frame_started.elapsed().as_secs_f32();
                frame_started = Instant::now();
                tick += elapsed * (0.05 / 0.033);
                frame_count = frame_count.wrapping_add(1);

                // Snake head position is tracked as a float so travel speed is
                // independent of the tick rate and of the LED count.
                if debug == DebugPattern::Snake {
                    if debug_jump {
                        // User picked an index: jump the head there and hold.
                        snake_pos = debug_index as f32;
                        if let Ok(mut cfg) = config_clone.lock() {
                            cfg.debug_jump = false;
                        }
                    } else if !debug_paused {
                        snake_pos += SNAKE_LEDS_PER_SEC * elapsed;
                        let span = led_count.max(1) as f32;
                        if snake_pos >= span {
                            snake_pos -= span;
                        }
                        // Report the head so the UI readout stays in sync.
                        debug_index = snake_pos as u32;
                    }
                } else {
                    // Other patterns use the index verbatim (e.g. Single).
                    snake_pos = debug_index as f32;
                }

                // Advance the marquee using real elapsed time, so scroll speed is
                // independent of the tick rate.
                if mode == AppMode::Text {
                    marquee_scroll += marquee_speed * elapsed;
                }

                // Keep the games' boards the same shape as the panel, then advance
                // them by real elapsed time so play speed is tick-rate independent.
                // (columns, rows) - see the note where `games` is created.
                games.sync_shape(
                    layout.lanes.max(1) as usize,
                    layout.leds_per_lane.max(1) as usize,
                );
                // Applies the user's speed multiplier and restarts the boards when
                // it actually changes.
                games.set_speed(game_speed);
                match mode {
                    AppMode::Snake => games.snake.step(elapsed),
                    AppMode::Tetris => games.tetris.step(elapsed),
                    _ => {}
                }

                // 1. Ensure OpenRGB Connection
                if openrgb_client.is_none() {
                    // Attempt connection once per second (every 30 frames)
                    if frame_count % 30 == 0 {
                        match OpenRgbClient::connect("127.0.0.1", ORGB_DEFAULT_PORT) {
                            Ok(mut client) => {
                                let devs = initialize_devices(&mut client, led_count, false);
                                log::info!("Connected to OpenRGB! Found {} controllers", devs.len());
                                devices_cache = devs;
                                openrgb_client = Some(client);
                                last_sent.clear();
                            }
                            Err(_) => {
                                // Silently wait for next attempt
                            }
                        }
                    }
                } else if let Some(ref mut client) = openrgb_client {
                    // Resize only when the user actually changes the LED count.
                    if led_count != last_requested_led_count {
                        if last_requested_led_count != 0 {
                            log::info!("LED count changed to {}, resizing zones...", led_count);
                            devices_cache = initialize_devices(client, led_count, true);
                            last_sent.clear();
                        }
                        last_requested_led_count = led_count;
                    } else if devices_cache.is_empty() || (frame_count % 150 == 0) {
                        // Periodic refresh or initial device populate
                        if let Ok(count) = client.get_controller_count() {
                            if count > 0 && devices_cache.len() != count as usize {
                                let devs = initialize_devices(client, led_count, false);
                                if !devs.is_empty() {
                                    log::info!("OpenRGB device cache refreshed: {} controllers active", devs.len());
                                    devices_cache = devs;
                                }
                            }
                        }
                    }
                }

                // 2. Poll sensors and the foreground window once per second.
                // These are the expensive OS queries; at 30 Hz they dominated
                // background CPU for no perceptible benefit.
                if last_sensor_poll.elapsed() >= Duration::from_secs(1) {
                    last_sensor_poll = Instant::now();
                    let (usage, temp) = sensors.get_metrics();
                    cpu_usage = usage;
                    cpu_temp = temp;
                    window_info = window_tracker.get_active_window_info();
                    app_color =
                        extract_app_color(&window_info.process_name, &window_info.title);
                }

                // 3. Compute target colors based on Mode
                let effective_mode = match mode {
                    AppMode::Smart => {
                        if window_info.is_coding {
                            AppMode::Coding
                        } else if window_info.is_gaming
                            || cpu_temp.is_some_and(|t| t > 72.0)
                        {
                            AppMode::Gaming
                        } else {
                            AppMode::Breathing
                        }
                    }
                    _ => mode.clone(),
                };

                // Only keep the audio capture stream open for modes that use it.
                // Smart mode is included because it falls through to the
                // Visualizer path when music is detected.
                let needs_audio = matches!(mode, AppMode::Visualizer | AppMode::Smart);
                audio_clone.set_needed(needs_audio);

                let audio_bands = audio_clone.get_bands();

                // Push bands to the UI at full tick rate. Polling could only
                // deliver ~4 Hz, which made the meter look like a 1-2 Hz animation
                // even though the FFT runs at 30 Hz.
                if needs_audio {
                    let _ = app.emit("audio-bands", &audio_bands);
                }

                // Push the game frames the same way. The UI preview must show the
                // real board - polling the whole state payload would arrive at
                // 4 Hz and the preview would visibly lag the panel.
                let game_kind = match mode {
                    AppMode::Snake => Some(crate::games::GameKind::Snake),
                    AppMode::Tetris => Some(crate::games::GameKind::Tetris),
                    _ => None,
                };
                if let Some(kind) = game_kind {
                    let snapshot = games.snapshot(kind);
                    let _ = app.emit("game-frame", &snapshot);
                }

                let (output_colors, current_rgb) = generate_frame(
                    &effective_mode,
                    led_count,
                    brightness,
                    &static_color,
                    &app_color,
                    cpu_temp,
                    tick * speed,
                    &audio_bands,
                    &layout,
                    viz_style,
                    &mut peaks,
                    (&marquee_text, marquee_scroll),
                    &mut games,
                );

                // 4. Send colors to OpenRGB hardware if connected
                let is_connected = openrgb_client.is_some();
                if let Some(ref mut client) = openrgb_client {
                    let mut failed = false;
                    for dev in &devices_cache {
                        let dev_led_count = if dev.led_count > 0 { dev.led_count } else { led_count };
                        scratch.clear();

                        // Warn when the device reports more LEDs than the grid
                        // covers, since the extra ones will simply stay dark.
                        if dev_led_count > led_count {
                            log::warn!(
                                "Device {} reports {} LEDs but the grid only covers {}; \
                                 the extra LEDs will be dark. Set the LED count to match \
                                 the physical strip, or lower the header length in OpenRGB.",
                                dev.id,
                                dev_led_count,
                                led_count
                            );
                        }

                        if debug != DebugPattern::Off {
                            // Diagnostic takes over the whole device so the physical
                            // layout can be read off the hardware directly.
                            scratch.extend(debug_frame(
                                &debug,
                                debug_index,
                                snake_pos,
                                dev_led_count as usize,
                            ));
                        } else if mode == AppMode::Off {
                            scratch.resize(dev_led_count as usize, RgbColor::new(0, 0, 0));
                        } else {
                            // output_colors is already indexed by RAW STRIP INDEX, so each
                            // device LED takes the colour for its own index directly.
                            //
                            // It must NOT be resampled to the device length. A zone can
                            // report more LEDs than the fan physically has (an oversized
                            // or stale header length), and resampling then compressed the
                            // frame so that `fan_idx * len / strip_len` resolved the first
                            // several LEDs all to index 0 - the unwired slot - leaving them
                            // dark, and only the opening colours ever reached the strip.
                            let smd_zone = dev.zones.len() > 1 && dev.zones[0].leds_count == 1;
                            let black = RgbColor::new(0, 0, 0);
                            for i in 0..dev_led_count {
                                if i == 0 && smd_zone {
                                    // Motherboard SMD LED mirrors the dominant colour
                                    scratch.push(current_rgb);
                                } else {
                                    let raw = if smd_zone { i.saturating_sub(1) } else { i };
                                    // Anything past the configured grid stays dark.
                                    scratch.push(
                                        output_colors
                                            .get(raw as usize)
                                            .copied()
                                            .unwrap_or(black),
                                    );
                                }
                            }
                        }

                        // Skip the TCP round-trip when this device's frame is unchanged.
                        // Static/Off modes settle to zero traffic per frame.
                        let unchanged = last_sent
                            .iter()
                            .find(|(id, _)| *id == dev.id)
                            .map(|(_, prev)| prev == &scratch)
                            .unwrap_or(false);
                        if unchanged {
                            continue;
                        }

                        if let Err(e) = client.update_leds(dev.id, &scratch) {
                            log::warn!("OpenRGB send error on device {}: {}", dev.id, e);
                            failed = true;
                            break;
                        }
                        match last_sent.iter_mut().find(|(id, _)| *id == dev.id) {
                            Some(entry) => entry.1.clone_from(&scratch),
                            None => last_sent.push((dev.id, scratch.clone())),
                        }
                    }
                    if failed {
                        log::warn!("OpenRGB connection lost, resetting client");
                        openrgb_client = None;
                        devices_cache.clear();
                        last_sent.clear();
                    }
                }

                // 5. Update State Payload for UI.
                // Devices are only deep-cloned when the controller list actually
                // changes, since DeviceInfo carries several heap strings.
                if let Ok(mut payload) = payload_clone.lock() {
                    payload.connected = is_connected;
                    payload.active_mode = mode;
                    payload.brightness = brightness;
                    payload.static_color = static_color;
                    payload.current_color = current_rgb;
                    payload.cpu_usage = cpu_usage;
                    payload.cpu_temp = cpu_temp;
                    payload.active_app = if window_info.process_name.is_empty() {
                        "Desktop".into()
                    } else {
                        window_info.process_name.clone()
                    };
                    payload.is_coding_detected = window_info.is_coding;
                    payload.is_gaming_detected = window_info.is_gaming;
                    if payload.devices.len() != devices_cache.len()
                        || payload
                            .devices
                            .iter()
                            .zip(devices_cache.iter())
                            .any(|(a, b)| a.id != b.id || a.led_count != b.led_count)
                    {
                        payload.devices = devices_cache.clone();
                        last_sent.clear();
                    }
                    payload.led_count = led_count;
                    payload.audio_bands = audio_bands;
                    payload.audio_sensitivity = audio_clone.get_sensitivity();
                    payload.app_color = app_color;
                    payload.debug = debug;
                    payload.debug_index = debug_index;
                    payload.debug_paused = debug_paused;
                    payload.layout = layout;
                    payload.viz_style = viz_style;
                    payload.marquee_text = marquee_text.clone();
                    payload.game_speed = game_speed;
                }
            }
        });
    }
}

fn initialize_devices(client: &mut OpenRgbClient, led_count: u32, resize: bool) -> Vec<DeviceInfo> {
    let count = client.get_controller_count().unwrap_or(0);
    let mut devs = Vec::new();
    for i in 0..count {
        if let Ok(mut info) = client.get_controller_data(i) {
            client.set_custom_mode(i).ok();

            for z in &info.zones {
                log::info!(
                    "Device {} '{}' zone {} '{}': {} LEDs (range {}..{})",
                    i,
                    info.name,
                    z.id,
                    z.name,
                    z.leds_count,
                    z.leds_min,
                    z.leds_max
                );
            }

            // Resize only when the user explicitly changes the LED count.
            // Previously every addressable zone was forced to the configured
            // value on connect, so a shorter physical strip latched only the
            // first few colours and the rest of the ramp never appeared.
            let mut resized = false;
            if resize {
                for zone in &info.zones {
                    if zone.leds_count > 0 && zone.leds_count != led_count {
                        let within_range = zone.leds_max == 0
                            || (led_count >= zone.leds_min && led_count <= zone.leds_max);
                        if within_range {
                            client.resize_zone(i, zone.id, led_count).ok();
                            resized = true;
                        }
                    }
                }
            }
            if resized {
                if let Ok(updated_info) = client.get_controller_data(i) {
                    info = updated_info;
                }
            }
            devs.push(info);
        }
    }
    devs
}

// Thermal gradient calculation: 40C (Cool Cyan) -> 55C (Green) -> 70C (Amber) -> 85C+ (Blazing Red)
fn temp_to_color(temp: f32) -> RgbColor {
    let t = temp.clamp(35.0, 85.0);
    if t < 50.0 {
        // Cyan (0, 220, 255) to Green (0, 255, 100)
        let ratio = (t - 35.0) / 15.0;
        let r = 0;
        let g = (220.0 + 35.0 * ratio) as u8;
        let b = (255.0 * (1.0 - ratio) + 100.0 * ratio) as u8;
        RgbColor::new(r, g, b)
    } else if t < 68.0 {
        // Green (0, 255, 100) to Yellow/Amber (255, 190, 0)
        let ratio = (t - 50.0) / 18.0;
        let r = (255.0 * ratio) as u8;
        let g = (255.0 - 65.0 * ratio) as u8;
        let b = (100.0 * (1.0 - ratio)) as u8;
        RgbColor::new(r, g, b)
    } else {
        // Yellow/Amber (255, 190, 0) to Intense Red (255, 10, 20)
        let ratio = ((t - 68.0) / 17.0).min(1.0);
        let r = 255;
        let g = (190.0 * (1.0 - ratio) + 10.0 * ratio) as u8;
        let b = (20.0 * ratio) as u8;
        RgbColor::new(r, g, b)
    }
}

/// LEDs per second the snake pulse travels. Slow enough to follow visually and
/// to read the index off the UI as it passes.
const SNAKE_LEDS_PER_SEC: f32 = 12.0;

/// How fast a visualizer peak marker falls, in normalized units per second.
/// The rise is instant; only the fall is damped, so it reports genuine maxima.
const PEAK_DECAY_PER_SEC: f32 = 0.55;

/// Faint output below this (at full brightness) is snapped to 0.
///
/// ARGB LEDs have poor low-level linearity: a channel driven at only a few counts
/// out of 255 still emits a visible glow, especially in a dark room. Without this,
/// LEDs that should be off show a faint ghost of their bar colour.
///
/// The threshold is scaled by the current brightness. A fixed cutoff would clip
/// different channels of the same colour at low brightness (cyan reduced to blue,
/// for example), distorting the hue; scaling it keeps the ratio between channels.
const LED_DEADZONE: f32 = 6.0;

/// Diagnostic palette for the counting pattern.
/// Paints 10 LEDs at a time in a repeating, unmistakable order:
///   1-10 RED, 11-20 GREEN, 21-30 BLUE, 31-40 WHITE, then repeats.
/// Count the LEDs in the first red run to read the real physical length, and
/// note where the pattern restarts to see how the strip is wired/channelled.
pub fn probe_color(i: usize) -> RgbColor {
    match i % 40 {
        0..=9 => RgbColor::new(255, 0, 0),     // RED
        10..=19 => RgbColor::new(0, 255, 0),   // GREEN
        20..=29 => RgbColor::new(0, 0, 255),   // BLUE
        _ => RgbColor::new(255, 255, 255),     // WHITE
    }
}

/// Build one diagnostic frame.
///
/// Each pattern is chosen to answer a specific hardware question:
///   `AllWhite` - how many LEDs respond at all
///   `Count`    - count runs of 10 to get the exact length
///   `Snake`    - walk a pulse one LED at a time to map index -> physical LED
///   `Single`   - light exactly one LED: if N other LEDs also light, those are
///                physically wired in parallel rather than daisy-chained
///   `Tens`     - a numbered ruler that survives losing your place
///   `Halves`   - detect mirroring between the two ends
pub fn debug_frame(
    pattern: &DebugPattern,
    index: u32,
    head: f32,
    count: usize,
) -> Vec<RgbColor> {
    const DIM: RgbColor = RgbColor { r: 0, g: 0, b: 0 };
    let white = RgbColor::new(255, 255, 255);
    let red = RgbColor::new(255, 0, 0);
    let blue = RgbColor::new(0, 0, 255);
    let n = count.max(1);

    match pattern {
        DebugPattern::Off => Vec::new(),
        DebugPattern::AllWhite => vec![white; count],
        DebugPattern::Count => (0..count).map(probe_color).collect(),
        DebugPattern::Snake => {
            // Tail length scales with the strip so short fans still show a clear
            // direction, and long strips don't wash out.
            let tail = (count as f32 / 6.0).clamp(3.0, 10.0);
            let head_pos = head.rem_euclid(n as f32);

            (0..count)
                .map(|i| {
                    let behind = (head_pos - i as f32).rem_euclid(n as f32);
                    let pulse = if behind < tail {
                        1.0 - behind / tail
                    } else {
                        0.0
                    };

                    // Dim ruler on every 10th LED, so the current position can be
                    // counted even while the snake is moving.
                    let ruler = if i % 10 == 0 { 0.12 } else { 0.0 };

                    if pulse > 0.01 {
                        // Head is white-hot and falls off through cyan to blue,
                        // which makes the direction of travel obvious.
                        let l = pulse;
                        RgbColor::new(
                            (255.0 * l * l * l) as u8,
                            (255.0 * l * l) as u8,
                            (255.0 * l) as u8,
                        )
                    } else {
                        let g = (ruler * 255.0) as u8;
                        RgbColor::new(g, g, g)
                    }
                })
                .collect()
        }
        DebugPattern::Single => {
            let target = (index as usize).min(count.saturating_sub(1));
            (0..count)
                .map(|i| if i == target { white } else { DIM })
                .collect()
        }
        DebugPattern::Tens => (0..count)
            .map(|i| {
                if i % 10 == 0 {
                    white
                } else if i % 5 == 0 {
                    RgbColor::new(40, 40, 40)
                } else {
                    DIM
                }
            })
            .collect(),
        DebugPattern::Halves => {
            let mid = count / 2;
            (0..count)
                .map(|i| if i < mid { red } else { blue })
                .collect()
        }
    }
}

/// Render scrolling text onto the panel.
///
/// Text is UPRIGHT and reads vertically: a glyph is 3 pixels wide (one per lane)
/// by N pixels tall, so characters stack down the panel and scroll upward. Each
/// letter therefore reads normally, unlike a rotated layout.
///
/// `scroll` is the offset in rows; it wraps so the message loops with a blank gap
/// between repeats.
pub fn marquee_frame(
    text: &str,
    scroll: f32,
    layout: &PanelLayout,
    led_count: u32,
    color: RgbColor,
    brightness: f32,
) -> Vec<RgbColor> {
    let rows = layout.leds_per_lane.max(1) as usize;

    // Build the message bottom-up: for each character, its rows from bottom to
    // top, then a blank gap. Concatenating in order means advancing through the
    // template moves UP the message, which is the scroll direction.
    let mut template: Vec<u8> = Vec::new();
    for ch in text.chars() {
        template.extend(font::glyph_rows_bottom_up(ch));
        for _ in 0..font::GLYPH_GAP {
            template.push(0);
        }
    }

    // Blank gap as tall as the panel, so the message fully clears before repeating.
    if !template.is_empty() {
        template.extend(std::iter::repeat(0u8).take(rows));
    }

    if template.is_empty() {
        return render_grid(layout, led_count, |_r, _c| RgbColor::new(0, 0, 0));
    }

    let period = template.len();
    let offset = (scroll.floor() as isize).rem_euclid(period as isize) as usize;
    let lit = apply_brightness(&color, brightness);
    let off = RgbColor::new(0, 0, 0);

    render_grid(layout, led_count, |row, col| {
        // One glyph row per panel row; its 3 bits map onto the 3 lanes.
        let glyph_row = template[(row as usize + offset) % period];
        if (glyph_row >> col) & 1 == 1 {
            lit
        } else {
            off
        }
    })
}

fn apply_brightness(color: &RgbColor, brightness: f32) -> RgbColor {
    let b = brightness.clamp(0.0, 1.0);

    // The cutoff scales with brightness, so it never changes the RATIO between
    // channels. A fixed cutoff would turn dim cyan into blue, for instance.
    // Below it, the whole colour drops to black - so an "off" LED is genuinely dark
    // instead of faintly glowing at the hardware's imperfect low end.
    if color.r.max(color.g).max(color.b) as f32 * b < LED_DEADZONE {
        return RgbColor::new(0, 0, 0);
    }

    RgbColor::new(
        (color.r as f32 * b) as u8,
        (color.g as f32 * b) as u8,
        (color.b as f32 * b) as u8,
    )
}

/// Bass-to-treble colour sweep, generated in HSL at FULL saturation so these
/// modes are as vivid as the rainbow effect.
///
/// This used to be a hand-picked RGB ramp which included a washed-out violet stop
/// (140, 60, 255). Next to the rainbow mode's fully saturated hues it looked
/// desaturated, and the effect got worse the brighter the strip was driven.
/// 340 (deep pink) -> 160 (mint), passing through magenta, violet and cyan.
fn spectrum_color(t: f32) -> RgbColor {
    let hue = 340.0 - t.clamp(0.0, 1.0) * 180.0;
    hsl_to_rgb(hue, 1.0, 0.5)
}

/// Blend a colour toward white.
///
/// Bar tips and peak markers used to be pure white. At low brightness that read as
/// a subtle highlight, but at high brightness those LEDs dominated the strip and
/// made the whole lane look washed out. A partial blend keeps them legible while
/// preserving the bar's colour.
fn mix_white(color: &RgbColor, amount: f32) -> RgbColor {
    let a = amount.clamp(0.0, 1.0);
    let f = |c: u8| (c as f32 + (255.0 - c as f32) * a) as u8;
    RgbColor::new(f(color.r), f(color.g), f(color.b))
}

fn generate_frame(
    mode: &AppMode,
    led_count: u32,
    brightness: f32,
    static_color: &RgbColor,
    app_color: &RgbColor,
    cpu_temp: Option<f32>,
    tick: f32,
    audio_bands: &[f32],
    layout: &PanelLayout,
    viz_style: VizStyle,
    peaks: &mut Vec<f32>,
    marquee: (&str, f32),
    games: &mut crate::games::Games,
) -> (Vec<RgbColor>, RgbColor) {
    let rows = layout.leds_per_lane.max(1);
    let cols = layout.lanes.max(1);

    // Render a game frame. Cells come back as palette indices, mapped to colours
    // here so the games stay free of brightness and colour concerns. Both games
    // are reached through the `Board` trait, so this is written once.
    let render_game = |board: &dyn crate::games::Board, brightness: f32| {
        let (lanes, board_rows) = board.dims();
        render_grid(layout, led_count, |row, col| {
            let (r, c) = (row as usize, col as usize);
            if r >= board_rows || c >= lanes {
                return RgbColor::new(0, 0, 0);
            }
            match board.cell(r, c) {
                Some(i) => apply_brightness(&crate::games::game_color(i, *static_color), brightness),
                // Empty cells are truly dark - no ambient floor, which would show
                // as ghosting on the hardware.
                None => RgbColor::new(0, 0, 0),
            }
        })
    };

    match mode {
        AppMode::AppSync => {
            // Gentle vertical wave in the foreground app's brand colour. Defined
            // per row so it reads as one continuous effect across all lanes.
            let base_color = apply_brightness(app_color, brightness);
            let colors = render_grid(layout, led_count, |row, _col| {
                let t = row as f32 / rows as f32;
                let wave = 0.82 + 0.18 * ((t * 6.28 + tick * 2.0).sin());
                apply_brightness(&base_color, wave)
            });
            (colors, base_color)
        }
        AppMode::Coding => {
            // Calm, low glare arctic focus tone, capped at 35% to protect night vision.
            let base_color = RgbColor::new(10, 140, 240);
            let c = apply_brightness(&base_color, (brightness * 0.35).min(0.35));
            let colors = render_grid(layout, led_count, |_r, _c| c);
            (colors, c)
        }
        AppMode::Gaming | AppMode::Thermal => {
            // Thermal mode needs a real reading. With no sensor available the honest
            // output is dark, not a synthesised "temperature" - the previous version
            // invented one from CPU load, so this gauge was never measuring anything.
            let Some(cpu_temp) = cpu_temp else {
                let zero = RgbColor::new(0, 0, 0);
                let colors = render_grid(layout, led_count, |_r, _c| zero);
                return (colors, zero);
            };

            // Thermometer: the strip fills from the bottom as temperature rises,
            // so height directly reads as heat. Fits the vertical lanes exactly.
            let temp_color = temp_to_color(cpu_temp);
            let pulse = 0.85 + 0.15 * (tick * 4.0).sin();

            // Map 35..85C onto the full height.
            let fill = ((cpu_temp.clamp(35.0, 85.0) - 35.0) / 50.0) * rows as f32;
            let lit = apply_brightness(&temp_color, brightness * pulse);
            // Unlit part of the column must be fully dark, otherwise the whole
            // strip glows faintly all the time.
            let unlit = RgbColor::new(0, 0, 0);
            let head = apply_brightness(&mix_white(&temp_color, 0.6), brightness * 0.55);

            // Highest lit row, used as the bright tip of the column.
            let tip = fill.floor() as u32;
            let frac = fill - tip as f32;

            let colors = render_grid(layout, led_count, |row, _col| {
                if row < tip {
                    lit
                } else if row == tip {
                    // Blend in the tip so the level moves smoothly.
                    apply_brightness(&head, frac.max(0.25))
                } else {
                    unlit
                }
            });
            (colors, lit)
        }
        AppMode::Breathing => {
            // Uniform pulse - already correct for a grid, no per-index logic needed.
            let breath = (tick.sin() + 1.0) * 0.5;
            let c = apply_brightness(static_color, brightness * (0.15 + 0.85 * breath));
            let colors = render_grid(layout, led_count, |_r, _c| c);
            (colors, c)
        }
        AppMode::Rainbow => {
            // Horizontal hue bands scrolling vertically. Each band spans all lanes,
            // so it stays continuous instead of restarting per lane.
            let colors = render_grid(layout, led_count, |row, _col| {
                let hue = ((row as f32 / rows as f32) * 360.0 + tick * 60.0) % 360.0;
                apply_brightness(&hsl_to_rgb(hue, 1.0, 0.5), brightness)
            });
            let dominant = apply_brightness(
                &hsl_to_rgb((tick * 60.0) % 360.0, 1.0, 0.5),
                brightness,
            );
            (colors, dominant)
        }
        AppMode::Static => {
            let c = apply_brightness(static_color, brightness);
            let colors = render_grid(layout, led_count, |_r, _c| c);
            (colors, c)
        }
        AppMode::Text => {
            let (text, scroll) = marquee;
            let colors = marquee_frame(
                text,
                scroll,
                layout,
                led_count,
                *static_color,
                brightness,
            );
            let dominant = apply_brightness(static_color, brightness);
            (colors, dominant)
        }
        AppMode::Snake => {
            let colors = render_game(&games.snake, brightness);
            let head = apply_brightness(&mix_white(static_color, 0.75), brightness);
            (colors, head)
        }
        AppMode::Tetris => {
            let colors = render_game(&games.tetris, brightness);
            let accent = apply_brightness(static_color, brightness);
            (colors, accent)
        }
        AppMode::Off => {
            let zero = RgbColor::new(0, 0, 0);
            let colors = render_grid(layout, led_count, |_r, _c| zero);
            (colors, zero)
        }
        AppMode::Visualizer => {
            let bands_len = audio_bands.len().max(1);
            let overall: f32 = if audio_bands.is_empty() {
                0.0
            } else {
                audio_bands.iter().sum::<f32>() / audio_bands.len() as f32
            };

            if overall < 0.02 {
                // Silence: output nothing at all. Falling back to the accent
                // colour made the LEDs glow when no audio was playing.
                for p in peaks.iter_mut() {
                    *p = 0.0;
                }
                let colors = render_grid(layout, led_count, |_r, _c| RgbColor::new(0, 0, 0));
                return (colors, RgbColor::new(0, 0, 0));
            }

            // Split the bands into `cols` contiguous musical ranges, which is now
            // meaningful because the bands are log-spaced by octave. Grouping by
            // linear index previously gave lane 1 the bass, lane 2 the mids and
            // lane 3 everything above 6 kHz - which is silence in most music, so
            // the third lane never lit.
            let mut levels = vec![0.0f32; cols as usize];
            for (c, lvl) in levels.iter_mut().enumerate() {
                let start = c * bands_len / cols as usize;
                let end = (((c + 1) * bands_len / cols as usize).max(start + 1)).min(bands_len);
                *lvl = audio_bands[start..end]
                    .iter()
                    .copied()
                    .fold(0.0f32, f32::max);
            }

            // Peak-hold: rise instantly, fall slowly. This is real signal
            // measurement, not decoration - it shows recent maxima.
            let decay = PEAK_DECAY_PER_SEC * (1.0 / 30.0);
            if peaks.len() < levels.len() {
                peaks.resize(levels.len(), 0.0);
            }
            for (i, &lvl) in levels.iter().enumerate() {
                peaks[i] = if lvl >= peaks[i] {
                    lvl
                } else {
                    (peaks[i] - decay).max(lvl)
                };
            }

            let dominant = apply_brightness(
                &spectrum_color(0.0),
                (brightness * (0.4 + 0.6 * levels[0])).clamp(0.0, 1.0),
            );

            let colors = match viz_style {
                // One vertical bar per lane. Bars fill bottom-to-top with a
                // travelling tip and a floating peak marker.
                VizStyle::Columns => render_grid(layout, led_count, |row, col| {
                    let lvl = levels[col as usize % levels.len()];
                    let peak = peaks[col as usize % peaks.len()];
                    let height = lvl * rows as f32;
                    let peak_row = peak * rows as f32;

                    // Span the palette fully across the lanes. Using `col / cols`
                    // only ever reached 2/3 of the ramp, so the last lane sat in
                    // the desaturated end of the range.
                    let t = if cols > 1 {
                        col as f32 / (cols - 1) as f32
                    } else {
                        0.0
                    };
                    let base = spectrum_color(t);

                    if (row as f32) < height - 1.0 {
                        apply_brightness(&base, brightness)
                    } else if (row as f32) < height {
                        // Bright tip. Tinted rather than pure white, so it reads as
                        // a highlight without washing the lane out at high brightness.
                        apply_brightness(&mix_white(&base, 0.55), brightness)
                    } else if (row as f32) <= peak_row && (row as f32) > peak_row - 1.0 {
                        // Peak-hold marker.
                        apply_brightness(&mix_white(&base, 0.7), brightness * 0.8)
                    } else {
                        // Off. No ambient floor: anything above zero here shows up
                        // as faint ghosting on LEDs that should be dark.
                        RgbColor::new(0, 0, 0)
                    }
                }),
                // One row per band, lit fully across every lane, so each band reads
                // as a single horizontal line spanning the whole fan.
                VizStyle::Rows => {
                    let mut row_levels = vec![0.0f32; rows as usize];
                    for (r, lvl) in row_levels.iter_mut().enumerate() {
                        let start = r * bands_len / rows as usize;
                        let end = (((r + 1) * bands_len / rows as usize).max(start + 1))
                            .min(bands_len);
                        *lvl = audio_bands[start..end]
                            .iter()
                            .copied()
                            .fold(0.0f32, f32::max);
                    }
                    render_grid(layout, led_count, |row, _col| {
                        // Flip the spectrum vertically if requested, so the
                        // graph can be oriented either way.
                        let band_row = if layout.bass_at_top {
                            rows.saturating_sub(1).saturating_sub(row)
                        } else {
                            row
                        };
                        let lvl = row_levels[band_row as usize % row_levels.len()];
                        let t = band_row as f32 / rows as f32;
                        // Scale by level, capped at the global brightness. No floor:
                        // an idle row must go fully dark rather than glow faintly.
                        apply_brightness(&spectrum_color(t), brightness * lvl.clamp(0.0, 1.0))
                    })
                }
                // Brightness blooms from the centre row outward on each band's level.
                VizStyle::Bloom => {
                    let mid = (rows as f32 - 1.0) / 2.0;
                    render_grid(layout, led_count, |row, col| {
                        let lvl = levels[col as usize % levels.len()];
                        // 1.0 at the centre, falling to 0 at the edges.
                        let d = ((row as f32 - mid).abs() / mid.max(1.0)).clamp(0.0, 1.0);
                        let radial = 1.0 - d;
                        let v = (lvl * 1.3 - d * 0.9).clamp(0.0, 1.0);
                        let t = if cols > 1 {
                            col as f32 / (cols - 1) as f32
                        } else {
                            0.0
                        };
                        let amount = (v * (0.5 + 0.5 * radial)).clamp(0.0, 1.0);
                        apply_brightness(&spectrum_color(t), brightness * amount)
                    })
                }
            };

            (colors, dominant)
        }
        AppMode::Smart => {
            let overall_energy: f32 = if audio_bands.is_empty() {
                0.0
            } else {
                audio_bands.iter().sum::<f32>() / audio_bands.len() as f32
            };

            // In Smart mode, if music is playing, fall through to the visualizer.
            if overall_energy > 0.08 {
                return generate_frame(
                    &AppMode::Visualizer,
                    led_count,
                    brightness,
                    static_color,
                    app_color,
                    cpu_temp,
                    tick,
                    audio_bands,
                    layout,
                    viz_style,
                    peaks,
                    marquee,
                    games,
                );
            }

            // Otherwise breathe gently on the accent colour.
            let breath = (tick.sin() + 1.0) * 0.5;
            let c = apply_brightness(static_color, brightness * (0.2 + 0.8 * breath));
            let colors = render_grid(layout, led_count, |_r, _c| c);
            (colors, c)
        }
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> RgbColor {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_p, g_p, b_p) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    RgbColor::new(
        ((r_p + m) * 255.0) as u8,
        ((g_p + m) * 255.0) as u8,
        ((b_p + m) * 255.0) as u8,
    )
}

pub fn extract_app_color(process_name: &str, title: &str) -> RgbColor {
    let proc = process_name.to_lowercase();
    let tit = title.to_lowercase();

    if proc.contains("spotify") || tit.contains("spotify") {
        RgbColor::new(30, 215, 96) // Spotify Vibrant Green #1ED760
    } else if proc.contains("discord") || tit.contains("discord") {
        RgbColor::new(88, 101, 242) // Discord Blurple #5865F2
    } else if proc.contains("code") || proc.contains("cursor") || proc.contains("devenv") || proc.contains("visual studio") {
        RgbColor::new(0, 122, 204) // VS Code Blue #007ACC
    } else if proc.contains("chrome") {
        RgbColor::new(66, 133, 244) // Google Chrome Blue #4285F4
    } else if proc.contains("msedge") || proc.contains("edge") {
        RgbColor::new(0, 164, 239) // Microsoft Edge Cyan #00A4EF
    } else if proc.contains("firefox") {
        RgbColor::new(255, 113, 57) // Firefox Flame Orange #FF7139
    } else if proc.contains("steam") {
        RgbColor::new(0, 173, 238) // Steam Cyan #00ADEE
    } else if proc.contains("photoshop") {
        RgbColor::new(49, 168, 255) // Photoshop Cyan #31A8FF
    } else if proc.contains("illustrator") {
        RgbColor::new(255, 154, 0) // Illustrator Orange #FF9A00
    } else if proc.contains("premiere") {
        RgbColor::new(153, 153, 255) // Premiere Purple #9999FF
    } else if proc.contains("figma") {
        RgbColor::new(242, 78, 30) // Figma Coral #F24E1E
    } else if proc.contains("slack") {
        RgbColor::new(224, 30, 90) // Slack Aubergine Pink #E01E5A
    } else if proc.contains("obs64") || proc.contains("obs") {
        RgbColor::new(170, 0, 255) // OBS Studio Purple #AA00FF
    } else if proc.contains("valorant") || proc.contains("riot") {
        RgbColor::new(255, 70, 85) // Valorant Crimson Red #FF4655
    } else if proc.contains("telegram") {
        RgbColor::new(36, 161, 222) // Telegram Cyan Blue #24A1DE
    } else if proc.contains("blender") {
        RgbColor::new(245, 121, 42) // Blender Deep Orange #F5792A
    } else if proc.contains("youtube") || tit.contains("youtube") {
        RgbColor::new(255, 0, 0) // YouTube Pure Red #FF0000
    } else if proc.contains("netflix") || tit.contains("netflix") {
        RgbColor::new(229, 9, 20) // Netflix Red #E50914
    } else if proc.contains("twitch") || tit.contains("twitch") {
        RgbColor::new(145, 70, 255) // Twitch Purple #9146FF
    } else if proc.contains("windowsterminal") || proc.contains("powershell") || proc.contains("cmd") {
        RgbColor::new(0, 255, 128) // Terminal Matrix Mint #00FF80
    } else if proc.contains("notion") {
        RgbColor::new(235, 235, 240) // Notion Crisp Titanium
    } else {
        if proc.is_empty() {
            return RgbColor::new(0, 210, 255);
        }
        let mut hash: u32 = 5381;
        for b in proc.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(b as u32);
        }
        let hue = (hash % 360) as f32;
        hsl_to_rgb(hue, 0.95, 0.52)
    }
}
