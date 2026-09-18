use crate::audio::AudioVisualizer;
use crate::openrgb::{DeviceInfo, OpenRgbClient, RgbColor, ORGB_DEFAULT_PORT};
use crate::sensors::{get_active_window_info, HardwareSensors};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

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
    pub cpu_temp: f32,
    pub active_app: String,
    pub is_coding_detected: bool,
    pub is_gaming_detected: bool,
    pub devices: Vec<DeviceInfo>,
    pub led_count: u32,
    pub audio_bands: Vec<f32>,
    pub audio_sensitivity: f32,
    pub app_color: RgbColor,
}

pub struct EngineConfig {
    pub mode: AppMode,
    pub brightness: f32,
    pub static_color: RgbColor,
    pub led_count: u32,
    pub speed: f32,
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
            led_count: 120,                          // Default 120 LEDs to fill full ARGB fan daisy-chains
            speed: 1.0,
        };

        let initial_payload = AppStatePayload {
            connected: false,
            active_mode: AppMode::Smart,
            brightness: 0.8,
            static_color: RgbColor::new(0, 210, 255),
            current_color: RgbColor::new(0, 210, 255),
            cpu_usage: 0.0,
            cpu_temp: 45.0,
            active_app: "Desktop".into(),
            is_coding_detected: false,
            is_gaming_detected: false,
            devices: Vec::new(),
            led_count: 120,
            audio_bands: vec![0.0; 32],
            audio_sensitivity: 1.0,
            app_color: RgbColor::new(0, 210, 255),
        };

        Self {
            config: Arc::new(Mutex::new(initial_config)),
            state_payload: Arc::new(Mutex::new(initial_payload)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self, audio: Arc<AudioVisualizer>) {
        let config_clone = Arc::clone(&self.config);
        let payload_clone = Arc::clone(&self.state_payload);
        let running_clone = Arc::clone(&self.running);
        let audio_clone = Arc::clone(&audio);

        std::thread::spawn(move || {
            let mut sensors = HardwareSensors::new();
            let mut openrgb_client: Option<OpenRgbClient> = None;
            let mut devices_cache: Vec<DeviceInfo> = Vec::new();
            let mut tick: f32 = 0.0;
            let mut frame_count: u64 = 0;
            let mut last_resized_led_count: u32 = 120;

            while running_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(33)); // ~30 FPS smooth update
                tick += 0.05;
                frame_count = frame_count.wrapping_add(1);

                let (mode, brightness, static_color, led_count, speed) = {
                    let cfg = config_clone.lock().unwrap();
                    (cfg.mode.clone(), cfg.brightness, cfg.static_color.clone(), cfg.led_count, cfg.speed)
                };

                // 1. Ensure OpenRGB Connection
                if openrgb_client.is_none() {
                    // Attempt connection once per second (every 30 frames)
                    if frame_count % 30 == 0 {
                        match OpenRgbClient::connect("127.0.0.1", ORGB_DEFAULT_PORT) {
                            Ok(mut client) => {
                                let devs = initialize_devices(&mut client, led_count);
                                log::info!("Connected to OpenRGB! Found {} controllers", devs.len());
                                devices_cache = devs;
                                openrgb_client = Some(client);
                                last_resized_led_count = led_count;
                            }
                            Err(_) => {
                                // Silently wait for next attempt
                            }
                        }
                    }
                } else if let Some(ref mut client) = openrgb_client {
                    // Check if user changed LED count in UI -> resize zones dynamically
                    if led_count != last_resized_led_count {
                        log::info!("LED count changed to {}, resizing zones...", led_count);
                        devices_cache = initialize_devices(client, led_count);
                        last_resized_led_count = led_count;
                    } else if devices_cache.is_empty() || (frame_count % 150 == 0) {
                        // Periodic refresh or initial device populate
                        if let Ok(count) = client.get_controller_count() {
                            if count > 0 && devices_cache.len() != count as usize {
                                let devs = initialize_devices(client, led_count);
                                if !devs.is_empty() {
                                    log::info!("OpenRGB device cache refreshed: {} controllers active", devs.len());
                                    devices_cache = devs;
                                }
                            }
                        }
                    }
                }

                // 2. Poll Sensors (Every ~500ms equivalent or smooth sample)
                let (cpu_usage, cpu_temp) = sensors.get_metrics();
                let window_info = get_active_window_info();
                let app_color = extract_app_color(&window_info.process_name, &window_info.title);

                // 3. Compute target colors based on Mode
                let effective_mode = match mode {
                    AppMode::Smart => {
                        if window_info.is_coding {
                            AppMode::Coding
                        } else if window_info.is_gaming || cpu_temp > 72.0 {
                            AppMode::Gaming
                        } else {
                            AppMode::Breathing
                        }
                    }
                    _ => mode.clone(),
                };

                let audio_bands = audio_clone.get_bands();

                let (output_colors, current_rgb) = generate_frame(
                    &effective_mode,
                    led_count,
                    brightness,
                    &static_color,
                    &app_color,
                    cpu_temp,
                    tick * speed,
                    &audio_bands,
                );

                // 4. Send colors to OpenRGB hardware if connected
                let is_connected = openrgb_client.is_some();
                if let Some(ref mut client) = openrgb_client {
                    let mut failed = false;
                    for dev in &devices_cache {
                        let dev_led_count = if dev.led_count > 0 { dev.led_count } else { led_count };
                        let mut frame_colors = Vec::with_capacity(dev_led_count as usize);

                        if mode == AppMode::Off {
                            let zero = RgbColor::new(0, 0, 0);
                            frame_colors.resize(dev_led_count as usize, zero);
                        } else {
                            for i in 0..dev_led_count {
                                if i == 0 && dev.zones.len() > 1 && dev.zones[0].leds_count == 1 {
                                    // Motherboard SMD LED gets current solid/dominant RGB
                                    frame_colors.push(current_rgb.clone());
                                } else {
                                    let fan_idx = if dev.zones.len() > 1 && dev.zones[0].leds_count == 1 {
                                        (i - 1) as usize
                                    } else {
                                        i as usize
                                    };
                                    let color_idx = fan_idx % output_colors.len();
                                    frame_colors.push(output_colors[color_idx].clone());
                                }
                            }
                        }

                        if let Err(e) = client.update_leds(dev.id, &frame_colors) {
                            log::warn!("OpenRGB send error on device {}: {}", dev.id, e);
                            failed = true;
                            break;
                        }
                    }
                    if failed {
                        log::warn!("OpenRGB connection lost, resetting client");
                        openrgb_client = None;
                        devices_cache.clear();
                    }
                }

                // 5. Update State Payload for UI
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
                        window_info.process_name
                    };
                    payload.is_coding_detected = window_info.is_coding;
                    payload.is_gaming_detected = window_info.is_gaming;
                    payload.devices = devices_cache.clone();
                    payload.led_count = led_count;
                    payload.audio_bands = audio_bands;
                    payload.audio_sensitivity = audio_clone.get_sensitivity();
                    payload.app_color = app_color;
                }
            }
        });
    }
}

fn initialize_devices(client: &mut OpenRgbClient, led_count: u32) -> Vec<DeviceInfo> {
    let count = client.get_controller_count().unwrap_or(0);
    let mut devs = Vec::new();
    for i in 0..count {
        if let Ok(mut info) = client.get_controller_data(i) {
            client.set_custom_mode(i).ok();
            let mut resized = false;
            for zone in &info.zones {
                let is_addressable = zone.name.to_lowercase().contains("addressable")
                    || zone.name.to_lowercase().contains("header")
                    || zone.zone_type == 1
                    || (zone.id > 0 && zone.leds_count == 0);

                if is_addressable && zone.leds_count != led_count {
                    client.resize_zone(i, zone.id, led_count).ok();
                    resized = true;
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

fn apply_brightness(color: &RgbColor, brightness: f32) -> RgbColor {
    let b = brightness.clamp(0.0, 1.0);
    RgbColor::new(
        (color.r as f32 * b) as u8,
        (color.g as f32 * b) as u8,
        (color.b as f32 * b) as u8,
    )
}

fn generate_frame(
    mode: &AppMode,
    led_count: u32,
    brightness: f32,
    static_color: &RgbColor,
    app_color: &RgbColor,
    cpu_temp: f32,
    tick: f32,
    audio_bands: &[f32],
) -> (Vec<RgbColor>, RgbColor) {
    let count = led_count.max(1) as usize;
    let mut colors = Vec::with_capacity(count);

    match mode {
        AppMode::AppSync => {
            // Smooth ambient flowing wave in foreground app's signature brand color
            let base = apply_brightness(app_color, brightness);
            let mut dominant = base.clone();
            for i in 0..count {
                let wave = 0.82 + 0.18 * ((i as f32 / count as f32 * 6.28 + tick * 2.0).sin());
                let c = apply_brightness(&base, wave);
                if i == 0 {
                    dominant = c.clone();
                }
                colors.push(c);
            }
            (colors, dominant)
        }
        AppMode::Coding => {
            // Calm, low glare, deep arctic focus tone (dimmed to max 35%)
            let focus_base = RgbColor::new(10, 140, 240);
            let coding_brightness = (brightness * 0.35).min(0.35);
            let final_color = apply_brightness(&focus_base, coding_brightness);
            colors.resize(count, final_color.clone());
            (colors, final_color)
        }
        AppMode::Gaming | AppMode::Thermal => {
            let temp_color = temp_to_color(cpu_temp);
            // Slight pulse animation when running warm
            let pulse = 0.85 + 0.15 * (tick * 4.0).sin();
            let final_color = apply_brightness(&temp_color, brightness * pulse);

            // Flowing gradient along fan perimeter
            for i in 0..count {
                let offset_temp = cpu_temp + 3.0 * ((i as f32 / count as f32 * 6.28 + tick).sin());
                let c = apply_brightness(&temp_to_color(offset_temp), brightness * pulse);
                colors.push(c);
            }
            (colors, final_color)
        }
        AppMode::Breathing => {
            let breath = (tick.sin() + 1.0) * 0.5; // 0.0 to 1.0
            let effective_bright = brightness * (0.15 + 0.85 * breath);
            let c = apply_brightness(static_color, effective_bright);
            colors.resize(count, c.clone());
            (colors, c)
        }
        AppMode::Rainbow => {
            let mut dominant = RgbColor::new(255, 0, 0);
            for i in 0..count {
                let hue = (i as f32 / count as f32 * 360.0 + tick * 60.0) % 360.0;
                let c = apply_brightness(&hsl_to_rgb(hue, 1.0, 0.5), brightness);
                if i == 0 {
                    dominant = c.clone();
                }
                colors.push(c);
            }
            (colors, dominant)
        }
        AppMode::Static => {
            let c = apply_brightness(static_color, brightness);
            colors.resize(count, c.clone());
            (colors, c)
        }
        AppMode::Off => {
            let zero = RgbColor::new(0, 0, 0);
            colors.resize(count, zero.clone());
            (colors, zero)
        }
        AppMode::Visualizer => {
            let overall_energy: f32 = if audio_bands.is_empty() {
                0.0
            } else {
                audio_bands.iter().sum::<f32>() / audio_bands.len() as f32
            };

            let bass_energy = if audio_bands.len() >= 2 {
                (audio_bands[0] * 0.65 + audio_bands[1] * 0.35).clamp(0.0, 1.0)
            } else {
                0.0
            };

            if overall_energy < 0.02 {
                // Calm, ambient breathing pulse when audio is silent
                let breath = 0.15 + 0.10 * (tick * 1.5).sin();
                let c = apply_brightness(static_color, brightness * breath);
                colors.resize(count, c.clone());
                (colors, c)
            } else {
                // Active audio reactivity:
                // Motherboard SMD LED & Center ring pulse on bass beat
                let bass_pulse = (bass_energy * 1.5).min(1.0);
                let punch_r = ((static_color.r as f32 * (1.0 - bass_pulse * 0.5) + 255.0 * bass_pulse * 0.5) as u8).min(255);
                let punch_g = ((static_color.g as f32 * (1.0 - bass_pulse * 0.8)) as u8).min(255);
                let punch_b = ((static_color.b as f32 * (1.0 - bass_pulse * 0.3) + 240.0 * bass_pulse * 0.3) as u8).min(255);
                let dominant = apply_brightness(
                    &RgbColor::new(punch_r, punch_g, punch_b),
                    (brightness * (0.35 + 0.65 * bass_pulse)).clamp(0.0, 1.0),
                );

                // Fan segment size for daisy chains (e.g. 120 LEDs = 3 fans of 40 LEDs, or 2 of 60, or 1 of count)
                let fan_size = if count >= 120 {
                    40
                } else if count >= 90 {
                    30
                } else if count >= 60 {
                    30
                } else {
                    count.max(1)
                };

                let half_fan = (fan_size as f32 / 2.0).max(1.0);

                for i in 0..count {
                    let local_i = i % fan_size;
                    // Symmetrical arc: 0.0 at base to 1.0 at peak
                    let norm_pos = if (local_i as f32) < half_fan {
                        local_i as f32 / half_fan
                    } else {
                        (fan_size - local_i) as f32 / half_fan
                    };

                    let band_idx = ((norm_pos * 7.99) as usize).min(7);
                    let band_val = if band_idx < audio_bands.len() {
                        audio_bands[band_idx]
                    } else {
                        0.0
                    };

                    // Graphic EQ VU meter fill
                    let is_lit = norm_pos <= (band_val * 1.15 + 0.05);

                    if is_lit {
                        // Vibrant frequency spectrum palette
                        let band_color = match band_idx {
                            0 => RgbColor::new(255, 20, 90),   // Sub-bass Kick (Deep Neon Red)
                            1 => RgbColor::new(240, 30, 160),  // Bass (Vivid Magenta)
                            2 => RgbColor::new(170, 40, 255),  // Low-mid (Electric Violet)
                            3 => RgbColor::new(70, 90, 255),   // Mid (Royal Cobalt)
                            4 => RgbColor::new(0, 210, 255),   // High-mid (Cyber Cyan)
                            5 => RgbColor::new(0, 255, 180),   // Presence (Aquamarine)
                            6 => RgbColor::new(255, 210, 60),  // Brilliance (Golden Solar)
                            _ => RgbColor::new(255, 245, 220), // Ultra (Ice White)
                        };

                        let led_bright = (brightness * (0.35 + 0.65 * band_val + 0.2 * bass_energy)).clamp(0.0, 1.0);
                        colors.push(apply_brightness(&band_color, led_bright));
                    } else {
                        // Subtle ambient floor outline (5% brightness)
                        let bg_dim = apply_brightness(static_color, brightness * 0.05);
                        colors.push(bg_dim);
                    }
                }

                (colors, dominant)
            }
        }
        AppMode::Smart => {
            let overall_energy: f32 = if audio_bands.is_empty() {
                0.0
            } else {
                audio_bands.iter().sum::<f32>() / audio_bands.len() as f32
            };

            // In Smart mode, if music is playing, seamlessly transition to audio reactivity!
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
                );
            }

            let breath = (tick.sin() + 1.0) * 0.5;
            let effective_bright = brightness * (0.2 + 0.8 * breath);
            let c = apply_brightness(static_color, effective_bright);
            colors.resize(count, c.clone());
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
