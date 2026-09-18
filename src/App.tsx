import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Code,
  Gamepad2,
  Sliders,
  Radio,
  Thermometer,
  Zap,
  Volume2,
  RefreshCw,
  Cpu,
  Layers,
  Sparkles,
  SunMedium,
  Clock,
  Palette,
  Bookmark,
  Plus,
  Trash2,
  CheckCircle2,
  Wrench,
  Flame,
  Waves,
  Settings,
  PowerOff,
  ShieldCheck,
  Monitor,
  AppWindow,
} from "lucide-react";
import { TitleBar } from "./components/TitleBar";
import { SpotlightCard } from "./components/SpotlightCard";
import { TelemetrySparkline } from "./components/TelemetrySparkline";
import { RotaryKnob } from "./components/RotaryKnob";

interface RgbColor {
  r: number;
  g: number;
  b: number;
}

interface DeviceInfo {
  id: number;
  name: string;
  vendor: string;
  description: string;
  led_count: number;
}

interface AppStatePayload {
  connected: boolean;
  active_mode: string;
  brightness: number;
  static_color: RgbColor;
  current_color: RgbColor;
  cpu_usage: number;
  cpu_temp: number;
  active_app: string;
  is_coding_detected: boolean;
  is_gaming_detected: boolean;
  devices: DeviceInfo[];
  led_count: number;
  audio_bands: number[];
  audio_sensitivity?: number;
  app_color?: RgbColor;
}

interface Profile {
  id: string;
  name: string;
  mode: string;
  brightness: number;
  color: RgbColor;
  ledCount: number;
  audioSensitivity?: number;
  icon?: string;
}

const PRESET_COLORS: { name: string; hex: string; rgb: RgbColor }[] = [
  { name: "Cyber Cyan", hex: "#00d2ff", rgb: { r: 0, g: 210, b: 255 } },
  { name: "Neon Purple", hex: "#b026ff", rgb: { r: 176, g: 38, b: 255 } },
  { name: "Arctic Ice", hex: "#38bdf8", rgb: { r: 56, g: 189, b: 248 } },
  { name: "Matrix Emerald", hex: "#10b981", rgb: { r: 16, g: 185, b: 129 } },
  { name: "Tokyo Blaze", hex: "#f97316", rgb: { r: 249, g: 115, b: 22 } },
  { name: "Crimson Laser", hex: "#ef4444", rgb: { r: 239, g: 68, b: 68 } },
  { name: "Pure White", hex: "#ffffff", rgb: { r: 255, g: 255, b: 255 } },
];

const BUILT_IN_PROFILES: Profile[] = [
  {
    id: "profile-cyber",
    name: "Cyber Matrix",
    mode: "rainbow",
    brightness: 0.9,
    color: { r: 0, g: 210, b: 255 },
    ledCount: 120,
  },
  {
    id: "profile-coding",
    name: "Late Night Code",
    mode: "coding",
    brightness: 0.35,
    color: { r: 10, g: 140, b: 240 },
    ledCount: 120,
  },
  {
    id: "profile-thermal",
    name: "Thermal Guard",
    mode: "thermal",
    brightness: 0.85,
    color: { r: 249, g: 115, b: 22 },
    ledCount: 120,
  },
  {
    id: "profile-audio",
    name: "Audio Rave",
    mode: "visualizer",
    brightness: 0.85,
    color: { r: 176, g: 38, b: 255 },
    ledCount: 120,
  },
];

const rgbToHex = (r: number, g: number, b: number) => {
  return (
    "#" +
    [r, g, b]
      .map((x) => {
        const hex = Math.round(x).toString(16);
        return hex.length === 1 ? "0" + hex : hex;
      })
      .join("")
  );
};

const hexToRgb = (hex: string): RgbColor | null => {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null;
};

export default function App() {
  const [audioSensitivity, setAudioSensitivity] = useState<number>(() => {
    try {
      const saved =
        localStorage.getItem("betterrgb_settings") ||
        localStorage.getItem("aurasync_settings");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.audio_sensitivity !== undefined)
          return parsed.audio_sensitivity;
      }
    } catch {}
    return 1.0;
  });

  const [activeTab, setActiveTab] = useState<
    "lighting" | "diagnostics" | "profiles" | "settings"
  >(() => {
    try {
      const saved =
        localStorage.getItem("betterrgb_settings") ||
        localStorage.getItem("aurasync_settings");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.active_tab) return parsed.active_tab;
      }
    } catch {}
    return "lighting";
  });

  const [state, setState] = useState<AppStatePayload>(() => {
    try {
      const saved =
        localStorage.getItem("betterrgb_settings") ||
        localStorage.getItem("aurasync_settings");
      if (saved) {
        const parsed = JSON.parse(saved);
        const color = parsed.color || { r: 0, g: 210, b: 255 };
        return {
          connected: false,
          active_mode: parsed.mode || "smart",
          brightness: parsed.brightness !== undefined ? parsed.brightness : 0.8,
          static_color: color,
          current_color: color,
          cpu_usage: 0,
          cpu_temp: 45,
          active_app: "Desktop",
          is_coding_detected: false,
          is_gaming_detected: false,
          devices: [],
          led_count: parsed.led_count || 120,
          audio_bands: new Array(32).fill(0),
          audio_sensitivity:
            parsed.audio_sensitivity !== undefined
              ? parsed.audio_sensitivity
              : 1.0,
        };
      }
    } catch {}
    return {
      connected: false,
      active_mode: "smart",
      brightness: 0.8,
      static_color: { r: 0, g: 210, b: 255 },
      current_color: { r: 0, g: 210, b: 255 },
      cpu_usage: 0,
      cpu_temp: 45,
      active_app: "Desktop",
      is_coding_detected: false,
      is_gaming_detected: false,
      devices: [],
      led_count: 120,
      audio_bands: new Array(32).fill(0),
      audio_sensitivity: 1.0,
    };
  });

  const [notification, setNotification] = useState<string | null>(null);
  const [startingOpenRgb, setStartingOpenRgb] = useState(false);

  // Autostart setting state
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [loadingAutostart, setLoadingAutostart] = useState(false);

  // Telemetry Sparkline History Buffers
  const [tempHistory, setTempHistory] = useState<number[]>([
    45, 46, 45, 47, 46,
  ]);
  const [loadHistory, setLoadHistory] = useState<number[]>([
    10, 15, 12, 18, 14,
  ]);

  // QOL: User Custom Profiles Saved in localStorage
  const [customProfiles, setCustomProfiles] = useState<Profile[]>(() => {
    try {
      const saved =
        localStorage.getItem("betterrgb_custom_profiles") ||
        localStorage.getItem("aurasync_custom_profiles");
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  // QOL: Sleep / Auto-off Timer
  const [sleepTimerRemaining, setSleepTimerRemaining] = useState<number | null>(
    null,
  );

  const showToast = (msg: string) => {
    setNotification(msg);
    setTimeout(() => setNotification(null), 3800);
  };

  const handleSensitivityChange = (newSens: number) => {
    setAudioSensitivity(newSens);
    invoke("set_audio_sensitivity", { sensitivity: newSens }).catch(() => {});
  };

  // Check initial Windows startup state
  useEffect(() => {
    invoke<boolean>("get_autostart_status")
      .then((status) => setAutostartEnabled(status))
      .catch((err) => console.warn("Failed to check autostart status:", err));
  }, []);

  const handleToggleAutostart = async () => {
    setLoadingAutostart(true);
    const nextState = !autostartEnabled;
    try {
      await invoke("set_autostart", { enable: nextState });
      setAutostartEnabled(nextState);
      showToast(
        nextState
          ? "betterRGB will launch automatically at Windows startup"
          : "betterRGB removed from Windows startup",
      );
    } catch (e: any) {
      showToast(`Startup setting failed: ${e}`);
    } finally {
      setLoadingAutostart(false);
    }
  };

  const handleQuitApp = async () => {
    try {
      await invoke("quit_app");
    } catch (e: any) {
      showToast(`Quit error: ${e}`);
    }
  };

  // Restore saved preferences to Rust backend on initial boot
  useEffect(() => {
    try {
      const saved =
        localStorage.getItem("betterrgb_settings") ||
        localStorage.getItem("aurasync_settings");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.mode)
          invoke("set_mode", { mode: parsed.mode }).catch(() => {});
        if (parsed.brightness !== undefined)
          invoke("set_brightness", { brightness: parsed.brightness }).catch(
            () => {},
          );
        if (parsed.color)
          invoke("set_static_color", {
            r: parsed.color.r,
            g: parsed.color.g,
            b: parsed.color.b,
          }).catch(() => {});
        if (parsed.led_count)
          invoke("set_led_count", { count: parsed.led_count }).catch(() => {});
        if (parsed.audio_sensitivity !== undefined)
          invoke("set_audio_sensitivity", {
            sensitivity: parsed.audio_sensitivity,
          }).catch(() => {});
      }
    } catch {
      // Ignore
    }
  }, []);

  // Save current settings to localStorage whenever changed
  useEffect(() => {
    const settings = {
      mode: state.active_mode,
      brightness: state.brightness,
      color: state.static_color,
      led_count: state.led_count,
      audio_sensitivity: audioSensitivity,
      active_tab: activeTab,
    };
    localStorage.setItem("betterrgb_settings", JSON.stringify(settings));
  }, [
    state.active_mode,
    state.brightness,
    state.static_color,
    state.led_count,
    audioSensitivity,
    activeTab,
  ]);

  // Poll state from Rust backend every 100ms
  useEffect(() => {
    let isMounted = true;
    const interval = setInterval(async () => {
      try {
        const res = await invoke<AppStatePayload>("get_state");
        if (isMounted) {
          setState(res);
          // Append to sparkline histories
          setTempHistory((prev) => [...prev.slice(-24), res.cpu_temp]);
          setLoadHistory((prev) => [...prev.slice(-24), res.cpu_usage]);
        }
      } catch {
        // Dev server fallback
      }
    }, 100);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, []);

  // Sleep Timer Interval countdown
  useEffect(() => {
    if (sleepTimerRemaining === null) return;

    if (sleepTimerRemaining <= 0) {
      // Timer finished -> Turn off LEDs
      invoke("set_mode", { mode: "off" });
      showToast("Sleep timer complete: LEDs turned off");
      setSleepTimerRemaining(null);
      return;
    }

    const timer = setInterval(() => {
      setSleepTimerRemaining((prev) =>
        prev !== null && prev > 0 ? prev - 1 : null,
      );
    }, 60000); // Check every minute

    return () => clearInterval(timer);
  }, [sleepTimerRemaining]);

  const handleStartSleepTimer = (minutes: number) => {
    setSleepTimerRemaining(minutes);
    showToast(`Sleep timer set: LEDs will turn off in ${minutes} minutes`);
  };

  const handleCancelSleepTimer = () => {
    setSleepTimerRemaining(null);
    showToast("Sleep timer cancelled");
  };

  const handleSetMode = async (mode: string) => {
    try {
      await invoke("set_mode", { mode });
    } catch (e: any) {
      showToast(`Error: ${e}`);
    }
  };

  const handleBrightnessChange = async (
    e: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const val = parseFloat(e.target.value);
    try {
      await invoke("set_brightness", { brightness: val });
    } catch (e: any) {
      showToast(`Error: ${e}`);
    }
  };

  const handleColorSelect = async (c: RgbColor) => {
    try {
      await invoke("set_static_color", { r: c.r, g: c.g, b: c.b });
    } catch (e: any) {
      showToast(`Error: ${e}`);
    }
  };

  const handleHexColorInput = (hex: string) => {
    const rgb = hexToRgb(hex);
    if (rgb) {
      handleColorSelect(rgb);
    }
  };

  const handleLedCountChange = async (count: number) => {
    try {
      await invoke("set_led_count", { count });
    } catch (e: any) {
      showToast(`Error: ${e}`);
    }
  };

  const handleStartOpenRgb = async () => {
    try {
      setStartingOpenRgb(true);
      showToast("Launching OpenRGB background server...");
      const res = await invoke<string>("launch_openrgb_server");
      showToast(res);
    } catch (e: any) {
      showToast(`OpenRGB server: ${e}`);
    } finally {
      setStartingOpenRgb(false);
    }
  };

  const handleStopArmouryCrate = async () => {
    try {
      const res = await invoke<string>("stop_armoury_crate_lighting");
      showToast(res);
    } catch (e: any) {
      showToast(`Armoury Crate service: ${e}`);
    }
  };

  // QOL: Hardware Ping Pulse (flashes White then returns to current mode)
  const handlePingHardware = async () => {
    showToast("Pinging ARGB headers with White signal pulse...");
    try {
      await invoke("set_static_color", { r: 255, g: 255, b: 255 });
      await invoke("set_mode", { mode: "static" });
      setTimeout(() => {
        handleColorSelect(state.static_color);
        handleSetMode(state.active_mode);
      }, 700);
    } catch (e: any) {
      showToast(`Ping failed: ${e}`);
    }
  };

  const handleApplyProfile = (profile: Profile) => {
    handleSetMode(profile.mode);
    handleBrightnessChange({
      target: { value: profile.brightness.toString() },
    } as any);
    handleColorSelect(profile.color);
    handleLedCountChange(profile.ledCount);
    if (profile.audioSensitivity !== undefined) {
      handleSensitivityChange(profile.audioSensitivity);
    }
    showToast(`Profile applied: ${profile.name}`);
  };

  const handleSaveCurrentAsProfile = () => {
    const name = prompt("Enter a name for this custom lighting profile:");
    if (!name || !name.trim()) return;

    const newProfile: Profile = {
      id: `custom-${Date.now()}`,
      name: name.trim(),
      mode: state.active_mode,
      brightness: state.brightness,
      color: state.static_color,
      ledCount: state.led_count,
      audioSensitivity: audioSensitivity,
    };

    const updated = [...customProfiles, newProfile];
    setCustomProfiles(updated);
    localStorage.setItem("betterrgb_custom_profiles", JSON.stringify(updated));
    showToast(`Custom profile saved: ${newProfile.name}`);
  };

  const handleDeleteProfile = (id: string) => {
    const updated = customProfiles.filter((p) => p.id !== id);
    setCustomProfiles(updated);
    localStorage.setItem("betterrgb_custom_profiles", JSON.stringify(updated));
    showToast("Profile removed");
  };

  const currentColorHex = rgbToHex(
    state.current_color.r,
    state.current_color.g,
    state.current_color.b,
  );

  const staticColorHex = rgbToHex(
    state.static_color.r,
    state.static_color.g,
    state.static_color.b,
  );

  const isOff = state.active_mode === "off";

  const MODES_LIST = [
    {
      id: "smart",
      name: "Smart Autonomous",
      description:
        "Auto-detects active coding vs gaming and dynamically adjusts color tone & glare.",
      icon: Sparkles,
      color: "rgba(0, 210, 255, 0.16)",
      glow: "#00d2ff",
    },
    {
      id: "appsync",
      name: "Adaptive App Glow",
      description:
        "Extracts brand colors directly from the foreground app's logo/icon and illuminates your PC.",
      icon: AppWindow,
      color: "rgba(0, 210, 255, 0.16)",
      glow: "#00d2ff",
    },
    {
      id: "coding",
      name: "Arctic Code Focus",
      description:
        "Deep, glare-free arctic focus tone dimmed to 35% to protect night vision.",
      icon: Code,
      color: "rgba(56, 189, 248, 0.16)",
      glow: "#38bdf8",
    },
    {
      id: "thermal",
      name: "Thermal Sentinel",
      description:
        "Direct real-time hardware temperature heat-mapping from Cool Green to Blazing Red.",
      icon: Flame,
      color: "rgba(249, 115, 22, 0.16)",
      glow: "#f97316",
    },
    {
      id: "visualizer",
      name: "WASAPI Visualizer",
      description:
        "Ripples live system audio beats & frequency bands across all connected fans.",
      icon: Volume2,
      color: "rgba(168, 85, 247, 0.16)",
      glow: "#a855f7",
    },
    {
      id: "rainbow",
      name: "Spectrum Wave",
      description:
        "Continuous ultra-smooth 360-degree rainbow stream across all ARGB headers.",
      icon: Radio,
      color: "rgba(16, 185, 129, 0.16)",
      glow: "#10b981",
    },
    {
      id: "breathing",
      name: "Circadian Pulse",
      description:
        "Gentle rhythmic fading breath cycle on your chosen custom accent color.",
      icon: Waves,
      color: "rgba(234, 179, 8, 0.16)",
      glow: "#eab308",
    },
    {
      id: "static",
      name: "Static Custom Accent",
      description:
        "Locks solid custom neon hue across all motherboard zones and fan hubs.",
      icon: Palette,
      color: "rgba(255, 255, 255, 0.12)",
      glow: "#ffffff",
    },
  ];

  return (
    <div className="relative flex h-screen w-screen flex-col bg-[#07090e] text-slate-100 overflow-hidden font-sans">
      {/* Tactile Noise Texture Overlay */}
      <div className="noise-overlay" />

      {/* Dynamic Aurora Mesh Glow Blobs (Reacts to live RGB) */}
      <div className="pointer-events-none fixed inset-0 overflow-hidden z-0">
        <div
          className="absolute -top-36 left-1/4 h-96 w-96 rounded-full blur-[140px] opacity-25 transition-all duration-1000"
          style={{ backgroundColor: isOff ? "transparent" : currentColorHex }}
        />
        <div
          className="absolute -bottom-36 right-1/4 h-96 w-96 rounded-full blur-[160px] opacity-20 transition-all duration-1000"
          style={{ backgroundColor: isOff ? "transparent" : currentColorHex }}
        />
      </div>

      {/* Custom Draggable Frameless Titlebar */}
      <TitleBar
        connected={state.connected}
        currentColorHex={currentColorHex}
        isOff={isOff}
        onTogglePower={() => handleSetMode(isOff ? "smart" : "off")}
        onStartOpenRgb={handleStartOpenRgb}
        onStopArmoury={handleStopArmouryCrate}
        startingOpenRgb={startingOpenRgb}
        sleepTimerMinutes={sleepTimerRemaining}
        onCancelSleepTimer={handleCancelSleepTimer}
      />

      {/* Toast Notification */}
      <AnimatePresence>
        {notification && (
          <motion.div
            initial={{ opacity: 0, y: -16, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -16, scale: 0.95 }}
            className="absolute top-14 right-6 z-50 flex items-center gap-2.5 rounded-xl border border-cyan-400/30 bg-slate-900/95 px-4 py-2.5 text-xs font-medium shadow-2xl backdrop-blur-xl"
          >
            <Zap className="h-4 w-4 text-cyan-400 animate-pulse" />
            <span className="text-slate-200">{notification}</span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* App Workspace Body */}
      <div className="flex-1 flex overflow-hidden relative z-10">
        {/* Left Navigation & Telemetry Sidebar */}
        <aside className="w-80 border-r border-white/[0.08] bg-[#090d16]/70 backdrop-blur-2xl flex flex-col justify-between p-4 shrink-0 overflow-y-auto">
          <div className="flex flex-col gap-4">
            {/* Nav Switcher */}
            <div className="flex rounded-xl bg-slate-950/70 p-1 border border-white/[0.06]">
              <button
                onClick={() => setActiveTab("lighting")}
                className={`flex-1 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                  activeTab === "lighting"
                    ? "bg-cyan-500/20 text-cyan-300 shadow-[0_0_12px_rgba(0,210,255,0.2)] border border-cyan-500/30"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                Lighting
              </button>
              <button
                onClick={() => setActiveTab("profiles")}
                className={`flex-1 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                  activeTab === "profiles"
                    ? "bg-cyan-500/20 text-cyan-300 shadow-[0_0_12px_rgba(0,210,255,0.2)] border border-cyan-500/30"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                Profiles
              </button>
              <button
                onClick={() => setActiveTab("diagnostics")}
                className={`flex-1 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                  activeTab === "diagnostics"
                    ? "bg-cyan-500/20 text-cyan-300 shadow-[0_0_12px_rgba(0,210,255,0.2)] border border-cyan-500/30"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                Health
              </button>
              <button
                onClick={() => setActiveTab("settings")}
                className={`flex-1 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                  activeTab === "settings"
                    ? "bg-cyan-500/20 text-cyan-300 shadow-[0_0_12px_rgba(0,210,255,0.2)] border border-cyan-500/30"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                Settings
              </button>
            </div>

            {/* Context & Focused App Card */}
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/50 p-3.5 backdrop-blur-md">
              <div className="flex items-center justify-between text-[11px] font-medium text-slate-400 mb-1.5">
                <span className="flex items-center gap-1.5">
                  <Activity className="h-3.5 w-3.5 text-cyan-400" />
                  Active Window Sensing
                </span>
                <span
                  className={`px-1.5 py-0.5 rounded text-[10px] font-semibold border ${
                    state.is_coding_detected
                      ? "bg-sky-500/10 border-sky-500/30 text-sky-300"
                      : state.is_gaming_detected
                        ? "bg-red-500/10 border-red-500/30 text-red-300"
                        : "bg-white/[0.04] border-white/[0.08] text-slate-400"
                  }`}
                >
                  {state.is_coding_detected
                    ? "Code"
                    : state.is_gaming_detected
                      ? "Game"
                      : "Desktop"}
                </span>
              </div>
              <div className="text-xs font-semibold text-white truncate flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 truncate flex-1">
                  {state.is_coding_detected ? (
                    <Code className="h-4 w-4 text-sky-400 shrink-0" />
                  ) : state.is_gaming_detected ? (
                    <Gamepad2 className="h-4 w-4 text-red-400 shrink-0" />
                  ) : (
                    <Activity className="h-4 w-4 text-slate-400 shrink-0" />
                  )}
                  <span className="truncate">
                    {state.active_app || "Explorer / Desktop"}
                  </span>
                </div>
                {state.app_color && (
                  <div
                    className="h-2.5 w-2.5 rounded-full shrink-0 border border-white/20 transition-all duration-500"
                    title={`Extracted App Logo Color: ${rgbToHex(state.app_color.r, state.app_color.g, state.app_color.b)}`}
                    style={{
                      backgroundColor: rgbToHex(
                        state.app_color.r,
                        state.app_color.g,
                        state.app_color.b,
                      ),
                      boxShadow: `0 0 8px ${rgbToHex(state.app_color.r, state.app_color.g, state.app_color.b)}`,
                    }}
                  />
                )}
              </div>
            </div>

            {/* Live Telemetry: CPU Temp Sparkline */}
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/50 p-3.5 backdrop-blur-md flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                  <Thermometer className="h-3.5 w-3.5 text-amber-400" />
                  CPU Temperature
                </span>
                <span
                  className={`text-sm font-bold ${
                    state.cpu_temp > 75
                      ? "text-red-400"
                      : state.cpu_temp > 60
                        ? "text-amber-400"
                        : "text-emerald-400"
                  }`}
                >
                  {Math.round(state.cpu_temp)}°C
                </span>
              </div>
              <TelemetrySparkline
                data={tempHistory}
                color={
                  state.cpu_temp > 75
                    ? "#ef4444"
                    : state.cpu_temp > 60
                      ? "#f59e0b"
                      : "#10b981"
                }
                min={30}
                max={90}
                height={38}
              />
            </div>

            {/* Live Telemetry: CPU Load Sparkline */}
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/50 p-3.5 backdrop-blur-md flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                  <Cpu className="h-3.5 w-3.5 text-cyan-400" />
                  CPU Utilization
                </span>
                <span className="text-sm font-bold text-cyan-300">
                  {Math.round(state.cpu_usage)}%
                </span>
              </div>
              <TelemetrySparkline
                data={loadHistory}
                color="#00d2ff"
                min={0}
                max={100}
                height={38}
              />
            </div>

            {/* WASAPI Audio Realtime Spectrum Visualizer */}
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/50 p-3.5 backdrop-blur-md flex flex-col gap-2.5">
              <div className="flex items-center justify-between text-[11px] font-medium text-slate-400">
                <span className="flex items-center gap-1.5">
                  <Volume2 className="h-3.5 w-3.5 text-violet-400" />
                  WASAPI Audio Equalizer
                </span>
                <span className="text-[10px] font-semibold text-violet-400/90 bg-violet-500/10 px-1.5 py-0.5 rounded border border-violet-500/20">
                  32 Bands
                </span>
              </div>
              <div className="flex h-11 items-end gap-[2px] px-0.5">
                {state.audio_bands.map((band, idx) => {
                  const t = idx / Math.max(1, state.audio_bands.length - 1);
                  const hue = (330 - t * 275 + 360) % 360;
                  const barColor = `hsl(${hue}, 95%, 58%)`;
                  return (
                    <div
                      key={idx}
                      className="flex-1 rounded-[1px] bg-slate-800/60 relative overflow-hidden"
                      style={{ height: "100%" }}
                      title={`Band ${idx + 1}: ${Math.round(band * 100)}%`}
                    >
                      <div
                        className="w-full absolute bottom-0 rounded-[1px] transition-all duration-75"
                        style={{
                          height: `${Math.min(100, Math.max(6, band * 100))}%`,
                          backgroundColor: barColor,
                          boxShadow:
                            band > 0.3 ? `0 0 6px ${barColor}` : "none",
                        }}
                      />
                    </div>
                  );
                })}
              </div>

              {/* Rotary Audio Sensitivity Knob */}
              <div className="pt-2 border-t border-white/[0.06] flex items-center justify-center">
                <RotaryKnob
                  value={audioSensitivity}
                  min={0.2}
                  max={3.0}
                  step={0.05}
                  defaultValue={1.0}
                  label="Sensitivity"
                  unit="x"
                  accentColor="#a855f7"
                  onChange={handleSensitivityChange}
                />
              </div>
            </div>
          </div>

          {/* QOL: Quick Sleep Timer selector at bottom of sidebar */}
          <div className="pt-3 border-t border-white/[0.06] flex flex-col gap-1.5">
            <div className="flex items-center justify-between text-[11px] text-slate-400 font-medium">
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3 text-cyan-400" />
                Auto-Off Sleep Timer
              </span>
              {sleepTimerRemaining !== null && (
                <span className="text-[10px] font-semibold text-cyan-300">
                  {sleepTimerRemaining}m left
                </span>
              )}
            </div>
            <div className="grid grid-cols-4 gap-1">
              {[15, 30, 60, 120].map((mins) => (
                <button
                  key={mins}
                  onClick={() => handleStartSleepTimer(mins)}
                  className={`py-1 text-[10px] font-medium rounded border transition-all cursor-pointer ${
                    sleepTimerRemaining === mins
                      ? "border-cyan-400 bg-cyan-500/20 text-cyan-300"
                      : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                  }`}
                >
                  {mins >= 60 ? `${mins / 60}h` : `${mins}m`}
                </button>
              ))}
            </div>
          </div>
        </aside>

        {/* Right Main Content Panel */}
        <main className="flex-1 p-6 overflow-y-auto flex flex-col gap-6">
          {activeTab === "lighting" && (
            <>
              {/* Lighting Profiles Grid with Aceternity Spotlight Hover */}
              <div>
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
                      <Sparkles className="h-4 w-4 text-cyan-400" />
                      Dynamic Lighting Profiles
                    </span>
                    <span className="text-[10px] px-2 py-0.5 rounded-full bg-cyan-500/10 border border-cyan-500/20 text-cyan-400 font-semibold">
                      60 FPS Native Tick
                    </span>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-3.5">
                  {MODES_LIST.map((m) => {
                    const isActive = state.active_mode === m.id;
                    const IconComp = m.icon;
                    return (
                      <SpotlightCard
                        key={m.id}
                        active={isActive}
                        spotlightColor={m.color}
                        onClick={() => handleSetMode(m.id)}
                        className="p-4 cursor-pointer flex flex-col justify-between h-36"
                      >
                        <div>
                          <div className="flex items-center justify-between mb-2">
                            <div
                              className="p-2 rounded-xl transition-all"
                              style={{
                                backgroundColor: isActive
                                  ? `${m.glow}25`
                                  : "rgba(255,255,255,0.04)",
                                color: isActive ? m.glow : "#94a3b8",
                                boxShadow: isActive
                                  ? `0 0 16px ${m.glow}55`
                                  : "none",
                              }}
                            >
                              <IconComp className="h-5 w-5" />
                            </div>
                            {isActive && (
                              <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-cyan-400/15 border border-cyan-400/30 text-cyan-300 text-[10px] font-semibold">
                                <span className="h-1.5 w-1.5 rounded-full bg-cyan-400 animate-ping" />
                                Active
                              </div>
                            )}
                          </div>
                          <h3 className="font-bold text-sm text-white">
                            {m.name}
                          </h3>
                        </div>
                        <p className="text-[11px] text-slate-400 leading-snug line-clamp-2">
                          {m.description}
                        </p>
                      </SpotlightCard>
                    );
                  })}
                </div>
              </div>

              {/* Visualizer Mode: Dedicated Dynamics Console Banner */}
              {state.active_mode === "visualizer" && (
                <div className="glass-panel rounded-2xl p-5 border border-violet-500/25 bg-gradient-to-r from-violet-950/30 via-slate-900/60 to-purple-950/20 backdrop-blur-xl flex items-center justify-between shadow-[0_0_30px_rgba(168,85,247,0.08)]">
                  <div className="flex flex-col gap-1.5 max-w-lg">
                    <div className="flex items-center gap-2">
                      <div className="p-1.5 rounded-lg bg-violet-500/20 border border-violet-500/30 text-violet-400 shadow-[0_0_12px_rgba(168,85,247,0.3)]">
                        <Volume2 className="h-4 w-4" />
                      </div>
                      <span className="text-xs font-bold uppercase tracking-wider text-white">
                        Visualizer Dynamics & Audio Gain Boost
                      </span>
                    </div>
                    <p className="text-xs text-slate-300 leading-relaxed">
                      Fine-tune hardware audio sensitivity. Lower values
                      preserve headroom during high volume; higher values boost
                      responsiveness during quiet listening.
                    </p>
                  </div>
                  <RotaryKnob
                    value={audioSensitivity}
                    min={0.2}
                    max={3.0}
                    step={0.05}
                    defaultValue={1.0}
                    label="Sensitivity"
                    unit="x"
                    accentColor="#c084fc"
                    onChange={handleSensitivityChange}
                  />
                </div>
              )}

              {/* Master Hardware Console: Brightness, ARGB Header Size & Precision Colors */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-5">
                <div className="flex items-center justify-between border-b border-white/[0.06] pb-3">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-2">
                    <Sliders className="h-4 w-4 text-cyan-400" />
                    Master Hardware Console
                  </span>
                  <div className="text-xs text-slate-400 font-medium">
                    ARGB Headers:{" "}
                    <span className="text-cyan-300 font-bold">
                      120 LEDs / 361 Total
                    </span>
                  </div>
                </div>

                {/* Sliders Grid: Brightness & ARGB Header Count */}
                <div className="grid grid-cols-2 gap-6">
                  {/* Master Global Brightness */}
                  <div className="flex flex-col gap-2">
                    <div className="flex justify-between text-xs text-slate-300 font-medium">
                      <span className="flex items-center gap-1.5">
                        <SunMedium className="h-4 w-4 text-amber-400" />
                        Global Master Brightness
                      </span>
                      <span className="font-bold text-cyan-400">
                        {Math.round(state.brightness * 100)}%
                      </span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={state.brightness}
                      onChange={handleBrightnessChange}
                      className="w-full cursor-pointer"
                    />
                    <div className="flex justify-between text-[10px] text-slate-500">
                      <span>0% (Off)</span>
                      <span>50%</span>
                      <span>100% (Vibrant)</span>
                    </div>
                  </div>

                  {/* ARGB Header Size Slider & Quick Presets */}
                  <div className="flex flex-col gap-2">
                    <div className="flex justify-between text-xs text-slate-300 font-medium">
                      <span className="flex items-center gap-1.5">
                        <Layers className="h-4 w-4 text-cyan-400" />
                        Addressable Header Fill Length
                      </span>
                      <span className="font-bold text-cyan-400">
                        {state.led_count} LEDs per header
                      </span>
                    </div>
                    <input
                      type="range"
                      min="12"
                      max="240"
                      step="6"
                      value={state.led_count}
                      onChange={(e) =>
                        handleLedCountChange(parseInt(e.target.value) || 120)
                      }
                      className="w-full cursor-pointer"
                    />
                    <div className="flex items-center gap-1.5">
                      {[36, 60, 72, 120, 180].map((count) => (
                        <button
                          key={count}
                          onClick={() => handleLedCountChange(count)}
                          className={`flex-1 py-0.5 text-[10px] rounded border font-medium transition-all cursor-pointer ${
                            state.led_count === count
                              ? "border-cyan-400 bg-cyan-500/20 text-cyan-300"
                              : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                          }`}
                        >
                          {count}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                {/* Color Selection Palette & Hex Picker */}
                <div className="flex flex-col gap-3 pt-3 border-t border-white/[0.06]">
                  <div className="flex items-center justify-between text-xs text-slate-300 font-medium">
                    <span className="flex items-center gap-1.5">
                      <Palette className="h-4 w-4 text-cyan-400" />
                      Accent Color Palette & Custom Hex
                    </span>

                    {/* Native Color Picker & Hex Input */}
                    <div className="flex items-center gap-2">
                      <input
                        type="color"
                        value={staticColorHex}
                        onChange={(e) => handleHexColorInput(e.target.value)}
                        className="h-6 w-6 rounded border border-white/20 bg-transparent cursor-pointer overflow-hidden p-0"
                      />
                      <input
                        type="text"
                        value={staticColorHex.toUpperCase()}
                        onChange={(e) => handleHexColorInput(e.target.value)}
                        className="w-20 px-2 py-0.5 rounded bg-slate-950 border border-white/[0.1] text-xs font-mono text-cyan-300 text-center uppercase"
                        maxLength={7}
                      />
                    </div>
                  </div>

                  {/* Curated Luxury Neon Presets */}
                  <div className="grid grid-cols-7 gap-2.5">
                    {PRESET_COLORS.map((preset) => {
                      const isSelected =
                        state.static_color.r === preset.rgb.r &&
                        state.static_color.g === preset.rgb.g &&
                        state.static_color.b === preset.rgb.b;
                      return (
                        <button
                          key={preset.name}
                          onClick={() => handleColorSelect(preset.rgb)}
                          className={`flex flex-col items-center gap-1.5 p-2 rounded-xl border transition-all cursor-pointer ${
                            isSelected
                              ? "border-white bg-slate-800 shadow-[0_0_16px_rgba(255,255,255,0.2)]"
                              : "border-white/[0.06] bg-slate-950/60 hover:border-white/[0.15]"
                          }`}
                        >
                          <div
                            className="h-7 w-7 rounded-full shadow-inner transition-transform hover:scale-110"
                            style={{
                              backgroundColor: preset.hex,
                              boxShadow: isSelected
                                ? `0 0 12px ${preset.hex}`
                                : "none",
                            }}
                          />
                          <span className="text-[10px] text-slate-400 truncate w-full text-center">
                            {preset.name}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            </>
          )}

          {activeTab === "profiles" && (
            <div className="flex flex-col gap-6">
              {/* Profile Header & Save Current */}
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-base font-bold text-white">
                    Lighting Profile Manager
                  </h2>
                  <p className="text-xs text-slate-400">
                    Switch between tuned lighting setups or save your current
                    configuration.
                  </p>
                </div>
                <button
                  onClick={handleSaveCurrentAsProfile}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 border border-cyan-400/40 text-cyan-300 text-xs font-semibold shadow-lg transition-all cursor-pointer"
                >
                  <Plus className="h-4 w-4" />
                  Save Current As Profile
                </button>
              </div>

              {/* Built-in Profiles */}
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-1.5">
                  <Bookmark className="h-3.5 w-3.5 text-cyan-400" />
                  Curated System Presets
                </h3>
                <div className="grid grid-cols-2 gap-3.5">
                  {BUILT_IN_PROFILES.map((p) => (
                    <SpotlightCard
                      key={p.id}
                      onClick={() => handleApplyProfile(p)}
                      className="p-4 cursor-pointer flex items-center justify-between"
                    >
                      <div className="flex items-center gap-3">
                        <div
                          className="h-10 w-10 rounded-xl flex items-center justify-center text-slate-950 font-bold"
                          style={{
                            backgroundColor: rgbToHex(
                              p.color.r,
                              p.color.g,
                              p.color.b,
                            ),
                            boxShadow: `0 0 12px ${rgbToHex(p.color.r, p.color.g, p.color.b)}66`,
                          }}
                        >
                          <Sparkles className="h-5 w-5 text-slate-950" />
                        </div>
                        <div>
                          <h4 className="font-bold text-sm text-white">
                            {p.name}
                          </h4>
                          <p className="text-xs text-slate-400 capitalize">
                            Mode: {p.mode} • {Math.round(p.brightness * 100)}%
                            Brightness
                          </p>
                        </div>
                      </div>
                      <span className="text-xs text-cyan-400 font-semibold">
                        Apply
                      </span>
                    </SpotlightCard>
                  ))}
                </div>
              </div>

              {/* Custom User Saved Profiles */}
              <div>
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-1.5">
                  <Bookmark className="h-3.5 w-3.5 text-emerald-400" />
                  Your Custom Saved Profiles ({customProfiles.length})
                </h3>
                {customProfiles.length === 0 ? (
                  <div className="rounded-2xl border border-white/[0.06] bg-slate-950/40 p-8 text-center text-xs text-slate-500">
                    No custom profiles saved yet. Fine-tune your lighting in the
                    Lighting tab and click &quot;Save Current As Profile&quot;!
                  </div>
                ) : (
                  <div className="grid grid-cols-2 gap-3.5">
                    {customProfiles.map((p) => (
                      <SpotlightCard
                        key={p.id}
                        className="p-4 flex items-center justify-between"
                      >
                        <div
                          onClick={() => handleApplyProfile(p)}
                          className="flex items-center gap-3 flex-1 cursor-pointer"
                        >
                          <div
                            className="h-10 w-10 rounded-xl flex items-center justify-center text-slate-950 font-bold shrink-0"
                            style={{
                              backgroundColor: rgbToHex(
                                p.color.r,
                                p.color.g,
                                p.color.b,
                              ),
                              boxShadow: `0 0 12px ${rgbToHex(p.color.r, p.color.g, p.color.b)}66`,
                            }}
                          >
                            <Sparkles className="h-5 w-5 text-slate-950" />
                          </div>
                          <div className="truncate">
                            <h4 className="font-bold text-sm text-white truncate">
                              {p.name}
                            </h4>
                            <p className="text-xs text-slate-400 capitalize">
                              Mode: {p.mode} • {p.ledCount} LEDs
                            </p>
                          </div>
                        </div>

                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => handleApplyProfile(p)}
                            className="text-xs text-cyan-400 hover:text-cyan-300 font-semibold px-2 py-1 rounded bg-cyan-500/10 cursor-pointer"
                          >
                            Apply
                          </button>
                          <button
                            onClick={() => handleDeleteProfile(p.id)}
                            className="text-slate-500 hover:text-red-400 p-1 cursor-pointer"
                            title="Delete profile"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </div>
                      </SpotlightCard>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {activeTab === "diagnostics" && (
            <div className="flex flex-col gap-5">
              <div>
                <h2 className="text-base font-bold text-white">
                  Hardware Health & Diagnostics
                </h2>
                <p className="text-xs text-slate-400">
                  Real-time status of lighting interfaces, OpenRGB background
                  server, and conflict detection.
                </p>
              </div>

              {/* Status Grid */}
              <div className="grid grid-cols-2 gap-4">
                <SpotlightCard className="p-4 flex flex-col gap-3">
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-sm text-white flex items-center gap-2">
                      <Zap className="h-4 w-4 text-cyan-400" />
                      OpenRGB Local Server
                    </span>
                    <span
                      className={`text-xs px-2 py-0.5 rounded-full border font-semibold ${
                        state.connected
                          ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
                          : "border-amber-500/30 bg-amber-500/10 text-amber-300"
                      }`}
                    >
                      {state.connected
                        ? "Listening (Port 6742)"
                        : "Disconnected"}
                    </span>
                  </div>
                  <p className="text-xs text-slate-400 leading-relaxed">
                    Direct low-overhead TCP socket connection transmitting 60
                    FPS lighting animations to your motherboard controller.
                  </p>
                  <button
                    onClick={handleStartOpenRgb}
                    disabled={startingOpenRgb || state.connected}
                    className="mt-1 py-1.5 rounded-lg border border-cyan-500/30 bg-cyan-500/15 text-cyan-300 text-xs font-semibold hover:bg-cyan-500/25 disabled:opacity-40 cursor-pointer"
                  >
                    {state.connected
                      ? "Connected & Active"
                      : "Restart OpenRGB Server"}
                  </button>
                </SpotlightCard>

                <SpotlightCard className="p-4 flex flex-col gap-3">
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-sm text-white flex items-center gap-2">
                      <RefreshCw className="h-4 w-4 text-violet-400" />
                      Asus Armoury Crate Shield
                    </span>
                    <span className="text-xs px-2 py-0.5 rounded-full border border-violet-500/30 bg-violet-500/10 text-violet-300 font-semibold">
                      Shield Ready
                    </span>
                  </div>
                  <p className="text-xs text-slate-400 leading-relaxed">
                    If Asus LightingService locks the motherboard USB HID
                    device, use this to pause background conflict services.
                  </p>
                  <button
                    onClick={handleStopArmouryCrate}
                    className="mt-1 py-1.5 rounded-lg border border-violet-500/30 bg-violet-500/15 text-violet-300 text-xs font-semibold hover:bg-violet-500/25 cursor-pointer"
                  >
                    Pause Asus LightingService
                  </button>
                </SpotlightCard>
              </div>

              {/* Hardware Test Ping Card */}
              <div className="glass-panel rounded-2xl p-5 flex items-center justify-between">
                <div>
                  <h4 className="font-bold text-sm text-white flex items-center gap-2">
                    <Wrench className="h-4 w-4 text-cyan-400" />
                    Interactive Hardware Ping Pulse
                  </h4>
                  <p className="text-xs text-slate-400 mt-0.5">
                    Flashes all 361 LEDs in high-intensity white for 0.7s to
                    physically verify hardware communication.
                  </p>
                </div>
                <button
                  onClick={handlePingHardware}
                  className="px-4 py-2 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 border border-cyan-400/40 text-cyan-300 text-xs font-bold transition-all cursor-pointer"
                >
                  Send Test Ping Pulse
                </button>
              </div>

              {/* Connected Hardware Device Info */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-3">
                <span className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
                  <CheckCircle2 className="h-4 w-4 text-emerald-400" />
                  Detected Motherboard RGB Controllers
                </span>
                {state.devices.length === 0 ? (
                  <div className="text-xs text-slate-500">
                    Scanning for controllers...
                  </div>
                ) : (
                  state.devices.map((dev) => (
                    <div
                      key={dev.id}
                      className="flex items-center justify-between rounded-xl bg-slate-950/60 p-3 border border-white/[0.06]"
                    >
                      <div>
                        <div className="font-semibold text-xs text-white">
                          {dev.name}
                        </div>
                        <div className="text-[11px] text-slate-400">
                          ID #{dev.id} • {dev.vendor || "ASUS"} •{" "}
                          {dev.led_count} Total Addressable LEDs
                        </div>
                      </div>
                      <span className="text-[11px] px-2 py-0.5 rounded bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 font-medium">
                        Device Synchronized
                      </span>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {activeTab === "settings" && (
            <div className="flex flex-col gap-5">
              <div>
                <h2 className="text-base font-bold text-white flex items-center gap-2">
                  <Settings className="h-4 w-4 text-cyan-400" />
                  Application Settings & Startup
                </h2>
                <p className="text-xs text-slate-400">
                  Configure Windows boot behavior, background tray integration,
                  and system preferences.
                </p>
              </div>

              {/* Startup Toggle Card */}
              <div className="glass-panel rounded-2xl p-5 border border-white/[0.08] flex items-center justify-between">
                <div className="flex flex-col gap-1 max-w-lg">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-bold text-white">
                      Launch at Windows Startup
                    </span>
                    <span
                      className={`text-[10px] px-2 py-0.5 rounded-full border font-semibold ${
                        autostartEnabled
                          ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
                          : "border-slate-700 bg-slate-800/40 text-slate-400"
                      }`}
                    >
                      {autostartEnabled ? "Enabled" : "Disabled"}
                    </span>
                  </div>
                  <p className="text-xs text-slate-400 leading-relaxed">
                    Automatically starts betterRGB when you turn on or sign in
                    to your PC. Lighting effects and audio synchronization
                    initialize silently in the background.
                  </p>
                </div>

                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={autostartEnabled}
                    disabled={loadingAutostart}
                    onChange={handleToggleAutostart}
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-slate-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-500 hover:opacity-90 transition-colors"></div>
                </label>
              </div>

              {/* System Tray Info Card */}
              <div className="glass-panel rounded-2xl p-5 border border-white/[0.08] flex flex-col gap-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-bold text-white flex items-center gap-2">
                    <Monitor className="h-4 w-4 text-cyan-400" />
                    System Tray & Background Operation
                  </span>
                  <span className="text-[10px] px-2 py-0.5 rounded-full border border-cyan-500/30 bg-cyan-500/10 text-cyan-300 font-semibold">
                    Always On
                  </span>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed">
                  betterRGB lives in your Windows taskbar system tray
                  (bottom-right near your clock).
                </p>
                <div className="grid grid-cols-3 gap-3 pt-1">
                  <div className="rounded-xl bg-slate-950/60 p-3.5 border border-white/[0.06]">
                    <div className="text-xs font-semibold text-white mb-1">
                      Click Close (X)
                    </div>
                    <div className="text-[11px] text-slate-400 leading-relaxed">
                      Hides to system tray instead of closing, ensuring
                      uninterrupted ARGB lighting and audio reactivity.
                    </div>
                  </div>
                  <div className="rounded-xl bg-slate-950/60 p-3.5 border border-white/[0.06]">
                    <div className="text-xs font-semibold text-white mb-1">
                      Left-Click Tray Icon
                    </div>
                    <div className="text-[11px] text-slate-400 leading-relaxed">
                      Instantly toggles showing or hiding the betterRGB control
                      window.
                    </div>
                  </div>
                  <div className="rounded-xl bg-slate-950/60 p-3.5 border border-white/[0.06]">
                    <div className="text-xs font-semibold text-white mb-1">
                      Right-Click Tray Icon
                    </div>
                    <div className="text-[11px] text-slate-400 leading-relaxed">
                      Opens native quick actions: Open, Hide, Quick LED Power
                      Toggle, or Complete Exit.
                    </div>
                  </div>
                </div>
              </div>

              {/* About & Quit Application */}
              <div className="grid grid-cols-2 gap-4">
                <div className="glass-panel rounded-2xl p-5 border border-white/[0.08] flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-1.5 mb-2">
                      <ShieldCheck className="h-4 w-4 text-emerald-400" />
                      About betterRGB
                    </span>
                    <div className="text-sm font-semibold text-white">
                      betterRGB v0.1.0
                    </div>
                    <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                      Next-generation lightweight, low-overhead hardware RGB
                      lighting controller engineered with Tauri v2, Rust native
                      tick engine, and React.
                    </p>
                  </div>
                  <div className="text-[11px] text-slate-500 pt-2 border-t border-white/[0.06]">
                    Target Motherboard: ASUS PRIME B760M-A
                  </div>
                </div>

                <div className="glass-panel rounded-2xl p-5 border border-red-500/20 bg-red-950/10 flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold uppercase tracking-wider text-red-400 flex items-center gap-1.5 mb-2">
                      <PowerOff className="h-4 w-4" />
                      Quit Application
                    </span>
                    <p className="text-xs text-slate-300 leading-relaxed">
                      Need to shut down betterRGB completely? This will
                      terminate all background sync threads and remove the
                      system tray icon.
                    </p>
                  </div>
                  <button
                    onClick={handleQuitApp}
                    className="py-2 px-4 rounded-xl border border-red-500/40 bg-red-500/20 hover:bg-red-500/30 text-red-300 text-xs font-bold transition-all cursor-pointer flex items-center justify-center gap-2"
                  >
                    <PowerOff className="h-3.5 w-3.5" />
                    Terminate & Exit betterRGB
                  </button>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
