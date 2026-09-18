use sysinfo::{Components, System};
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

    pub fn get_metrics(&mut self) -> (f32, f32) {
        self.sys.refresh_cpu_usage();
        let cpu_usage = self.sys.global_cpu_usage();

        self.components.refresh(true);
        let mut max_temp = 0.0f32;
        for c in &self.components {
            let label = c.label().to_lowercase();
            if label.contains("cpu")
                || label.contains("core")
                || label.contains("package")
                || label.contains("gpu")
            {
                if let Some(temp) = c.temperature() {
                    if temp > max_temp {
                        max_temp = temp;
                    }
                }
            }
        }

        // Fallback estimation if raw component thermal diode is blocked by hypervisor/privileges
        if max_temp <= 1.0 {
            max_temp = 42.0 + (cpu_usage / 100.0) * 38.0;
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

pub fn get_active_window_info() -> ActiveWindowInfo {
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

        let mut process_name = String::new();
        if pid > 0 {
            let mut s = System::new();
            s.refresh_processes(
                sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
                true,
            );
            if let Some(proc_) = s.process(sysinfo::Pid::from_u32(pid)) {
                process_name = proc_.name().to_string_lossy().to_string();
            }
        }

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
