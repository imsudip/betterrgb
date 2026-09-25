use sysinfo::{Components, Pid, System};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

pub struct HardwareSensors {
    sys: System,
    components: Components,
}

impl HardwareSensors {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let components = Components::new_with_refreshed_list();

        Self { sys, components }
    }

    /// Read CPU load and package temperature.
    ///
    /// The temperature is `None` when no sensor is available. sysinfo exposes no
    /// components at all on many machines (it ships no CPU thermal driver), and the
    /// previous version silently substituted a value derived from CPU load - so the
    /// UI showed a plausible-looking number that was never measured.
    pub fn get_metrics(&mut self) -> (f32, Option<f32>) {
        self.sys.refresh_cpu_usage();
        let cpu_usage = self.sys.global_cpu_usage();

        self.components.refresh(false);

        // Only genuine CPU/package sensors. GPU labels are excluded: they used to be
        // included, so a hot graphics card could be shown as the CPU temperature.
        let mut max_temp: Option<f32> = None;
        for c in &self.components {
            let label = c.label().to_lowercase();
            let is_cpu = label.contains("cpu")
                || label.contains("core")
                || label.contains("package")
                || label.contains("tctl")
                || label.contains("tdie")
                || label.contains("k10temp")
                || label.contains("coretemp");

            if is_cpu {
                if let Some(t) = c.temperature() {
                    if t.is_finite() && t > 1.0 {
                        max_temp = Some(max_temp.map_or(t, |m: f32| m.max(t)));
                    }
                }
            }
        }

        (cpu_usage, max_temp)
    }
}

pub struct ActiveWindowInfo {
    #[allow(dead_code)]
    pub title: String,
    pub process_name: String,
    pub is_coding: bool,
    pub is_gaming: bool,
}

/// Tracks the foreground window and its process. Reuses one `System` and caches the
/// process name per PID, instead of building a new `System` on every call.
pub struct ActiveWindowTracker {
    sys: System,
    cached_pid: u32,
    cached_name: String,
}

impl ActiveWindowTracker {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            cached_pid: 0,
            cached_name: String::new(),
        }
    }

    pub fn get_active_window_info(&mut self) -> ActiveWindowInfo {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0 == 0 as _ {
                return ActiveWindowInfo {
                    title: String::new(),
                    process_name: String::new(),
                    is_coding: false,
                    is_gaming: false,
                };
            }

            let mut buffer = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buffer);
            let title = if len > 0 {
                String::from_utf16_lossy(&buffer[..len as usize])
            } else {
                String::new()
            };

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));

            // Only pay for a process lookup when the foreground process changes.
            if pid != self.cached_pid {
                self.cached_pid = pid;
                self.cached_name.clear();
                if pid > 0 {
                    let spid = Pid::from_u32(pid);
                    self.sys
                        .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[spid]), false);
                    if let Some(proc_) = self.sys.process(spid) {
                        self.cached_name = proc_.name().to_string_lossy().to_string();
                    }
                }
            }
            let process_name = self.cached_name.clone();

            let proc_lower = process_name.to_lowercase();
            let title_lower = title.to_lowercase();

            let coding_tokens = [
                "code.exe",
                "cursor.exe",
                "devenv.exe",
                "pycharm",
                "clion",
                "webstorm",
                "idea64",
                "sublime",
                "notepad++",
                "windowsterminal.exe",
                "powershell.exe",
                "cmd.exe",
                "github desktop",
                "visual studio",
                "neovim",
                "nvim",
            ];

            let gaming_tokens = [
                "steam.exe",
                "epicgameslauncher",
                "riotclient",
                "valorant",
                "cs2.exe",
                "overwatch",
                "dota2.exe",
                "league of legends",
                "cyberpunk2077",
                "rdr2",
                "gta5.exe",
                "cod.exe",
                "minecraft",
                "fortnite",
            ];

            let is_coding = coding_tokens
                .iter()
                .any(|&token| proc_lower.contains(token) || title_lower.contains(token));
            let is_gaming = gaming_tokens
                .iter()
                .any(|&token| proc_lower.contains(token) || title_lower.contains(token));

            ActiveWindowInfo {
                title,
                process_name,
                is_coding,
                is_gaming,
            }
        }
    }
}
