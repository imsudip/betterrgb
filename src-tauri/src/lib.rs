mod audio;
mod engine;
mod font;
mod games;
mod openrgb;
mod sensors;

use audio::AudioVisualizer;
use engine::{AppMode, AppStatePayload, DebugPattern, PanelLayout, RgbEngine, VizStyle};
use openrgb::RgbColor;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};

pub struct AppState {
    pub engine: Arc<RgbEngine>,
    pub audio: Arc<AudioVisualizer>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistentConfig {
    mode: String,
    brightness: f32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    led_count: u32,
    audio_sensitivity: f32,
    #[serde(default)]
    marquee_text: Option<String>,
    #[serde(default)]
    marquee_speed: Option<f32>,
    #[serde(default)]
    game_speed: Option<f32>,
}

fn get_config_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(&appdata).join("betterrgb");
        let new_file = dir.join("settings.json");
        if !new_file.exists() {
            let legacy_file = PathBuf::from(&appdata).join("aurasync").join("settings.json");
            if legacy_file.exists() {
                std::fs::create_dir_all(&dir).ok();
                let _ = std::fs::copy(&legacy_file, &new_file);
            }
        }
        std::fs::create_dir_all(&dir).ok();
        new_file
    } else {
        PathBuf::from("betterrgb_settings.json")
    }
}

fn save_persistent_config(state: &AppState) {
    let cfg = state.engine.config.lock().unwrap();
    let mode_str = match cfg.mode {
        AppMode::Smart => "smart",
        AppMode::Thermal => "thermal",
        AppMode::Visualizer => "visualizer",
        AppMode::Coding => "coding",
        AppMode::Gaming => "gaming",
        AppMode::Static => "static",
        AppMode::Breathing => "breathing",
        AppMode::Rainbow => "rainbow",
        AppMode::AppSync => "appsync",
        AppMode::Text => "text",
        AppMode::Snake => "snake",
        AppMode::Tetris => "tetris",
        AppMode::Off => "off",
    };
    let data = PersistentConfig {
        mode: mode_str.to_string(),
        brightness: cfg.brightness,
        color_r: cfg.static_color.r,
        color_g: cfg.static_color.g,
        color_b: cfg.static_color.b,
        led_count: cfg.led_count,
        audio_sensitivity: state.audio.get_sensitivity(),
        marquee_text: Some(cfg.marquee_text.clone()),
        marquee_speed: Some(cfg.marquee_speed),
        game_speed: Some(cfg.game_speed),
    };
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, json);
    }
}

fn load_persistent_config(engine: &Arc<RgbEngine>, audio: &Arc<AudioVisualizer>) {
    let path = get_config_path();
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<PersistentConfig>(&content) {
            let target_mode = match cfg.mode.to_lowercase().as_str() {
                "smart" => AppMode::Smart,
                "thermal" => AppMode::Thermal,
                "visualizer" => AppMode::Visualizer,
                "coding" => AppMode::Coding,
                "gaming" => AppMode::Gaming,
                "static" => AppMode::Static,
                "breathing" => AppMode::Breathing,
                "rainbow" => AppMode::Rainbow,
                "appsync" => AppMode::AppSync,
                "off" => AppMode::Off,
                _ => AppMode::Smart,
            };
            let color = RgbColor::new(cfg.color_r, cfg.color_g, cfg.color_b);
            let b = cfg.brightness.clamp(0.0, 1.0);
            let count = cfg.led_count.clamp(1, 1000);
            let sens = cfg.audio_sensitivity.clamp(0.1, 5.0);

            if let Ok(mut engine_cfg) = engine.config.lock() {
                engine_cfg.mode = target_mode.clone();
                engine_cfg.brightness = b;
                engine_cfg.static_color = color.clone();
                engine_cfg.led_count = count;
                if let Some(t) = cfg.marquee_text {
                    if !t.trim().is_empty() {
                        engine_cfg.marquee_text = t;
                    }
                }
                if let Some(s) = cfg.marquee_speed {
                    engine_cfg.marquee_speed = s.clamp(1.0, 60.0);
                }
                if let Some(g) = cfg.game_speed {
                    engine_cfg.game_speed = g.clamp(0.25, 4.0);
                }
            }
            if let Ok(mut payload) = engine.state_payload.lock() {
                payload.active_mode = target_mode;
                payload.brightness = b;
                payload.static_color = color.clone();
                payload.current_color = color;
                payload.led_count = count;
                payload.audio_sensitivity = sens;
            }
            audio.set_sensitivity(sens);
            log::info!("Loaded persistent AuraSync settings: mode={}, brightness={}, led_count={}, sensitivity={}", cfg.mode, b, count, sens);
        }
    }
}

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> AppStatePayload {
    let mut payload = state.inner().engine.state_payload.lock().unwrap().clone();
    payload.audio_bands = state.inner().audio.get_bands();
    payload.audio_sensitivity = state.inner().audio.get_sensitivity();
    payload
}

#[tauri::command]
fn set_mode(mode: String, state: State<'_, AppState>) -> Result<(), String> {
    let target_mode = match mode.to_lowercase().as_str() {
        "smart" => AppMode::Smart,
        "thermal" => AppMode::Thermal,
        "visualizer" => AppMode::Visualizer,
        "coding" => AppMode::Coding,
        "gaming" => AppMode::Gaming,
        "static" => AppMode::Static,
        "breathing" => AppMode::Breathing,
        "rainbow" => AppMode::Rainbow,
        "appsync" => AppMode::AppSync,
        "text" => AppMode::Text,
        "snake" => AppMode::Snake,
        "tetris" => AppMode::Tetris,
        "off" => AppMode::Off,
        other => return Err(format!("Unknown mode: {}", other)),
    };

    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.mode = target_mode;
    }
    save_persistent_config(state.inner());
    Ok(())
}

/// Set the message shown by Text mode. Filtered to characters the bitmap font
/// can actually render, and length-capped so the buffer stays bounded.
#[tauri::command]
fn set_marquee_text(text: String, state: State<'_, AppState>) -> Result<(), String> {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || " .,!?:-_+/'=():<>".contains(*c))
        .take(120)
        .collect();
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.marquee_text = cleaned;
    }
    save_persistent_config(state.inner());
    Ok(())
}

/// Set the marquee scroll speed in pixels per second.
#[tauri::command]
fn set_marquee_speed(speed: f32, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.marquee_speed = speed.clamp(1.0, 60.0);
    }
    save_persistent_config(state.inner());
    Ok(())
}

/// Set the play-speed multiplier for the Snake and Tetris demos.
/// 1.0 is the tuned default; higher is faster.
#[tauri::command]
fn set_game_speed(speed: f32, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.game_speed = speed.clamp(0.25, 4.0);
    }
    save_persistent_config(state.inner());
    Ok(())
}

#[tauri::command]
fn set_brightness(brightness: f32, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.brightness = brightness.clamp(0.0, 1.0);
    }
    save_persistent_config(state.inner());
    Ok(())
}

#[tauri::command]
fn set_static_color(r: u8, g: u8, b: u8, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.static_color = RgbColor::new(r, g, b);
    }
    save_persistent_config(state.inner());
    Ok(())
}

#[tauri::command]
fn set_led_count(count: u32, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.led_count = count.clamp(1, 1000);
    }
    save_persistent_config(state.inner());
    Ok(())
}

/// Set the diagnostic test pattern shown on the hardware (Debug page).
#[tauri::command]
fn set_debug_pattern(pattern: String, state: State<'_, AppState>) -> Result<(), String> {
    let parsed = match pattern.to_lowercase().as_str() {
        "off" => DebugPattern::Off,
        "all_white" => DebugPattern::AllWhite,
        "count" => DebugPattern::Count,
        "snake" => DebugPattern::Snake,
        "single" => DebugPattern::Single,
        "tens" => DebugPattern::Tens,
        "halves" => DebugPattern::Halves,
        other => return Err(format!("Unknown debug pattern: {}", other)),
    };
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.debug = parsed;
    }
    Ok(())
}

/// Set the focus LED index, and optionally pin the snake so it stops travelling.
/// Jumping to an index always pauses, so the LED you asked for stays lit.
#[tauri::command]
fn set_debug_index(
    index: u32,
    pause: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.debug_index = index.min(999);
        cfg.debug_jump = true;
        if let Some(p) = pause {
            cfg.debug_paused = p;
        }
    }
    Ok(())
}

/// Resume or pause the snake without changing its current position.
#[tauri::command]
fn set_debug_paused(paused: bool, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.debug_paused = paused;
    }
    Ok(())
}

/// Configure how the physical LEDs are arranged as a grid.
#[tauri::command]
fn set_lane_layout(
    lanes: u32,
    leds_per_lane: u32,
    first_index: u32,
    serpentine: bool,
    bass_at_top: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.layout = PanelLayout {
            lanes: lanes.clamp(1, 16),
            leds_per_lane: leds_per_lane.clamp(1, 120),
            first_index: first_index.min(63),
            serpentine,
            bass_at_top,
        };
    }
    Ok(())
}

/// Choose the audio visualizer rendering style.
#[tauri::command]
fn set_viz_style(style: String, state: State<'_, AppState>) -> Result<(), String> {
    let parsed = match style.to_lowercase().as_str() {
        "columns" => VizStyle::Columns,
        "rows" => VizStyle::Rows,
        "bloom" => VizStyle::Bloom,
        other => return Err(format!("Unknown viz style: {}", other)),
    };
    {
        let mut cfg = state.inner().engine.config.lock().unwrap();
        cfg.viz_style = parsed;
    }
    Ok(())
}

#[tauri::command]
fn set_audio_sensitivity(sensitivity: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.inner().audio.set_sensitivity(sensitivity);
    save_persistent_config(state.inner());
    Ok(())
}

/// Checks if OpenRGB server is currently active and reachable on localhost
pub fn is_openrgb_server_running() -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], openrgb::ORGB_DEFAULT_PORT)),
        std::time::Duration::from_millis(300),
    )
    .is_ok()
}

/// Dynamically locates the OpenRGB executable on any device, checking:
/// 1. Tauri bundled resource directory
/// 2. Current running executable directory & its ancestors
/// 3. Current working directory & its parent
/// 4. Common Windows installation directories (Program Files, LocalAppData)
/// 5. System PATH
pub fn find_openrgb_executable(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut search_dirs = Vec::new();

    // 1. App resource directory (bundled in installer/package)
    if let Ok(res_dir) = app.path().resource_dir() {
        search_dirs.push(res_dir.clone());
        search_dirs.push(res_dir.join("_up_"));
        search_dirs.push(res_dir.join("_up_").join("bin"));
        search_dirs.push(res_dir.join("_up_").join("bin").join("OpenRGB"));
        search_dirs.push(res_dir.join("bin"));
        search_dirs.push(res_dir.join("bin").join("OpenRGB"));
        search_dirs.push(res_dir.join("resources"));
    }

    // 2. Directory containing the running executable and ancestors (up to 6 levels)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut curr = exe_dir.to_path_buf();
            for _ in 0..6 {
                search_dirs.push(curr.clone());
                search_dirs.push(curr.join("_up_"));
                search_dirs.push(curr.join("_up_").join("bin"));
                search_dirs.push(curr.join("_up_").join("bin").join("OpenRGB"));
                search_dirs.push(curr.join("bin"));
                search_dirs.push(curr.join("bin").join("OpenRGB"));
                if !curr.pop() {
                    break;
                }
            }
        }
    }

    // 3. Current working directory and parent
    if let Ok(cwd) = std::env::current_dir() {
        search_dirs.push(cwd.clone());
        search_dirs.push(cwd.join("_up_").join("bin"));
        search_dirs.push(cwd.join("bin"));
        search_dirs.push(cwd.join("bin").join("OpenRGB"));
        if let Some(parent) = cwd.parent() {
            search_dirs.push(parent.to_path_buf());
            search_dirs.push(parent.join("bin"));
            search_dirs.push(parent.join("bin").join("OpenRGB"));
        }
    }

    // Candidate relative subpaths where OpenRGB may reside
    let relative_candidates = [
        r"_up_\bin\OpenRGB\OpenRGB.exe",
        r"_up_\bin\OpenRGB.exe",
        r"bin\OpenRGB\OpenRGB.exe",
        r"bin\OpenRGB.exe",
        r"OpenRGB\OpenRGB.exe",
        r"OpenRGB\OpenRGB Windows 64-bit\OpenRGB.exe",
        r"OpenRGB Windows 64-bit\OpenRGB.exe",
        r"openrgb\OpenRGB.exe",
        r"OpenRGB.exe",
    ];

    for base in &search_dirs {
        for rel in &relative_candidates {
            let candidate = base.join(rel);
            if candidate.is_file() {
                log::info!("Discovered OpenRGB at: {}", candidate.display());
                return Ok(candidate);
            }
        }
    }

    // 4. Direct scan in res_dir and exe_dir for bundled OpenRGB
    let root_dirs = [
        app.path().resource_dir().ok(),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())),
    ];
    for root in root_dirs.into_iter().flatten() {
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let c1 = p.join("OpenRGB.exe");
                    if c1.is_file() {
                        return Ok(c1);
                    }
                    let c2 = p.join("bin").join("OpenRGB.exe");
                    if c2.is_file() {
                        return Ok(c2);
                    }
                    let c3 = p.join("bin").join("OpenRGB").join("OpenRGB.exe");
                    if c3.is_file() {
                        return Ok(c3);
                    }
                }
            }
        }
    }

    // 4. Common Windows install locations
    let env_vars = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA", "APPDATA"];
    for var in &env_vars {
        if let Ok(dir_str) = std::env::var(var) {
            let base = PathBuf::from(dir_str);
            let candidates = [
                base.join(r"OpenRGB\OpenRGB.exe"),
                base.join(r"Programs\OpenRGB\OpenRGB.exe"),
            ];
            for candidate in &candidates {
                if candidate.is_file() {
                    log::info!("Discovered installed OpenRGB at: {}", candidate.display());
                    return Ok(candidate.to_path_buf());
                }
            }
        }
    }

    // 5. System PATH lookup
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join("OpenRGB.exe");
            if candidate.is_file() {
                log::info!("Discovered OpenRGB in PATH at: {}", candidate.display());
                return Ok(candidate);
            }
        }
    }

    Err("OpenRGB executable was not found. Please ensure OpenRGB is bundled in the bin folder or installed on the system.".into())
}

/// Spawns OpenRGB in headless background server mode with correct working directory
pub fn launch_openrgb_headless(app: &tauri::AppHandle) -> Result<String, String> {
    if is_openrgb_server_running() {
        return Ok("OpenRGB server is already active and listening".into());
    }

    let exe_path = find_openrgb_executable(app)?;
    let working_dir = exe_path
        .parent()
        .ok_or_else(|| "Could not determine OpenRGB directory".to_string())?;

    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(working_dir)
        .arg("--server")
        .arg("--server-port")
        .arg(openrgb::ORGB_DEFAULT_PORT.to_string());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW prevents command prompt window from flashing
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.spawn() {
        Ok(_) => {
            log::info!("Spawned OpenRGB server from {}", exe_path.display());
            Ok("OpenRGB background server started successfully".into())
        }
        Err(e) => Err(format!("Failed to start OpenRGB: {}", e)),
    }
}

#[tauri::command]
fn launch_openrgb_server(app: tauri::AppHandle) -> Result<String, String> {
    launch_openrgb_headless(&app)
}

#[tauri::command]
fn stop_armoury_crate_lighting() -> Result<String, String> {
    let cmd = "Stop-Service -Name 'LightingService' -Force -ErrorAction SilentlyContinue; Stop-Process -Name 'LightingService' -Force -ErrorAction SilentlyContinue";
    let mut command = Command::new("powershell");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command
        .args(&["-NoProfile", "-NonInteractive", "-Command", cmd])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok("LightingService paused".into())
    } else {
        Err("Could not pause LightingService (Administrator privileges might be required)".into())
    }
}

#[tauri::command]
fn get_autostart_status() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = Command::new("reg.exe")
            .creation_flags(0x08000000)
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "betterRGB",
            ])
            .output();
        if let Ok(out) = output {
            return out.status.success();
        }
    }
    false
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<bool, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if enable {
            let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let exe_path = current_exe.to_string_lossy().to_string();
            let reg_val = format!("\"{}\"", exe_path);

            let status = Command::new("reg.exe")
                .creation_flags(0x08000000)
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "betterRGB",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &reg_val,
                    "/f",
                ])
                .status()
                .map_err(|e| e.to_string())?;

            Ok(status.success())
        } else {
            let _ = Command::new("reg.exe")
                .creation_flags(0x08000000)
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "betterRGB",
                    "/f",
                ])
                .status();
            Ok(false)
        }
    }
    #[cfg(not(windows))]
    Ok(false)
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio = Arc::new(AudioVisualizer::new());
    let engine = Arc::new(RgbEngine::new());
    load_persistent_config(&engine, &audio);

    let engine_for_setup = Arc::clone(&engine);
    let audio_for_setup = Arc::clone(&audio);

    tauri::Builder::default()
        .manage(AppState {
            engine,
            audio,
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_mode,
            set_brightness,
            set_static_color,
            set_led_count,
            set_debug_pattern,
            set_debug_index,
            set_debug_paused,
            set_lane_layout,
            set_viz_style,
            set_marquee_text,
            set_marquee_speed,
            set_game_speed,
            set_audio_sensitivity,
            launch_openrgb_server,
            stop_armoury_crate_lighting,
            get_autostart_status,
            set_autostart,
            restart_app,
            quit_app
        ])
        .setup(move |app| {
            // Started here so the engine can emit UI events via the app handle.
            // Audio capture itself is lazy - the engine enables it only while a
            // mode that reacts to audio is active.
            engine_for_setup.start(Arc::clone(&audio_for_setup), app.handle().clone());
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // System Tray Menu & Setup
            let show_i = MenuItem::with_id(app, "show", "Open betterRGB", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Hide to Tray", true, None::<&str>)?;
            let power_i = MenuItem::with_id(app, "toggle_power", "Toggle Lighting On / Off", true, None::<&str>)?;
            let restart_i = MenuItem::with_id(app, "restart", "Restart betterRGB", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit betterRGB", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &hide_i, &power_i, &restart_i, &quit_i])?;

            if let Some(icon) = app.default_window_icon() {
                let tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip("betterRGB — Asus RGB Controller")
                    .on_menu_event(move |app, event| {
                        match event.id.as_ref() {
                            "restart" => {
                                app.restart();
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            }
                            "hide" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                            }
                            "toggle_power" => {
                                if let Some(state) = app.try_state::<AppState>() {
                                    let mut cfg = state.engine.config.lock().unwrap();
                                    if cfg.mode == AppMode::Off {
                                        cfg.mode = AppMode::Smart;
                                    } else {
                                        cfg.mode = AppMode::Off;
                                    }
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.unminimize();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    })
                    .build(app)?;

                std::mem::forget(tray);
            }

            // Keep running in background when window close button is clicked
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            // Auto-launch OpenRGB in background on startup if not already running
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(300));
                if !is_openrgb_server_running() {
                    match launch_openrgb_headless(&app_handle) {
                        Ok(msg) => log::info!("OpenRGB auto-start: {}", msg),
                        Err(e) => log::warn!("OpenRGB auto-start notice: {}", e),
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
