# betterRGB — Smart Hardware ARGB Controller

<div align="center">
  <img src="public/app-icon.png" alt="betterRGB Logo" width="128" height="128" />
  <h3>Next-generation lightweight, low-overhead ARGB lighting controller engineered with Tauri v2, Rust native tick engine, and React 19.</h3>

  <p>
    <img src="https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-blue?style=flat-square" alt="Platform" />
    <img src="https://img.shields.io/badge/Engine-Rust%20%2F%20Tauri%20v2-orange?style=flat-square" alt="Tauri v2" />
    <img src="https://img.shields.io/badge/Frontend-React%2019%20%2B%20Tailwind%20CSS-cyan?style=flat-square" alt="React 19" />
    <img src="https://img.shields.io/badge/Driver-OpenRGB%20SDK%20v0.9+-green?style=flat-square" alt="OpenRGB SDK" />
  </p>
</div>

---

## 🌟 Why betterRGB?

Manufacturer lighting suites like **Asus Armoury Crate** consume hundreds of megabytes of RAM, install over 30 background services, and frequently cause micro-stutters during gaming.

**betterRGB** replaces bloated vendor software with a single high-performance native desktop application:

- **< 25 MB RAM** average memory footprint.
- **0% CPU** idle overhead when backgrounded.
- **60 FPS ultra-smooth hardware tick loop** directly over OpenRGB's low-latency TCP socket.
- **No kernel-level anti-cheat conflicts** (compatible with Riot Vanguard, EasyAntiCheat, BattlEye).

---

## ✨ Features

### 🎨 Adaptive Foreground App Glow (`AppSync`)

- Automatically samples the active foreground application window and reflects its signature brand colors directly onto your fans and motherboard LEDs.
- Built-in signature brand dictionary for popular apps (Spotify `#1ED760`, Discord `#5865F2`, VS Code `#007ACC`, Chrome `#4285F4`, Edge `#00A4EF`, Firefox `#FF7139`, Steam `#00ADEE`, Photoshop `#31A8FF`, Figma `#F24E1E`, Valorant `#FF4655`, and many more).
- Instantaneous deterministic HSV color hashing for any uncatalogued desktop software.

### 🧠 Smart Autonomous Sensing (`Smart Auto`)

- **Coding Focus**: Automatically identifies active code editors and terminals (VS Code, Cursor, Visual Studio, JetBrains, Windows Terminal, Neovim) and smoothly dims the lighting to an eye-friendly, glare-free arctic ice tone (25–35% brightness).
- **Gaming & Workload Reaction**: Detects fullscreen games or heavy system workloads and transitions to thermal sentinel telemetry.
- **Ambient Breathing**: Rhythmic circadian breathing pulse when idling.

### 🎵 WASAPI Loopback Audio Visualizer

- Real-time 32-band hardware FFT frequency analyzer capturing master output via Windows Audio Session API (WASAPI loopback) without virtual audio cables.
- Rhythmic bass thumps pulse the motherboard SMD LEDs, while mid/high frequencies ripple down fan daisy-chains.
- Includes an analog rotary sensitivity knob with smooth deceleration physics.

### 🔥 Hardware Thermal Sentinel

- Direct real-time CPU thermal diode mapping:
  - **Cool (< 50°C)**: Electric Cyan & Mint
  - **Warm (50°C – 70°C)**: Amber & Sunset Orange
  - **Critical (> 75°C)**: Blazing Thermal Red Pulse

### 🪟 System Tray & Background Lifecycle

- Minimizing or clicking **Close (X)** silently hides betterRGB to the Windows taskbar system tray.
- **Left-Click Tray Icon**: Instantly toggles show / hide with window focus.
- **Right-Click Context Menu**:
  - _Open betterRGB_
  - _Hide to Tray_
  - _Toggle Lighting On / Off_
  - _Restart betterRGB_
  - _Quit betterRGB_

### 🚀 Windows Autostart on Boot

- Native Windows startup configuration under the **Settings** tab.
- Transparently configures `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` without flashing console windows or needing third-party startup helpers.

---

## 🛠️ Architecture & Tech Stack

```
┌─────────────────────────────────────────────────────────┐
│                      betterRGB UI                       │
│    React 19 • Tailwind CSS 4 • Framer Motion • Lucide   │
└────────────────────────────┬────────────────────────────┘
                             │ Tauri v2 IPC
┌────────────────────────────▼────────────────────────────┐
│                    Rust Native Core                     │
│  ├── 60 FPS Tick Engine (engine.rs)                     │
│  ├── WASAPI Loopback & FFT Analyzer (audio.rs)          │
│  ├── Win32 Sensor & Active Window Monitor (sensors.rs)  │
│  └── OpenRGB TCP Protocol Client (openrgb.rs)           │
└────────────────────────────┬────────────────────────────┘
                             │ TCP Localhost (Port 6742)
┌────────────────────────────▼────────────────────────────┐
│                      OpenRGB Server                     │
│         Motherboard SMBus / ARGB Fan Controller         │
└─────────────────────────────────────────────────────────┘
```

---

## 💻 Getting Started

### Prerequisites

- Windows 10 or 11 (64-bit)
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (1.77.2+)

### Development Mode

```powershell
# Install frontend dependencies
npm install

# Run in development mode with hot reload
npm run tauri dev
```

### Production Build & Installer

```powershell
# Build optimized binaries and NSIS/MSI installer
npm run tauri build
```

The compiled standalone executable and installer packages will be located in:

```
src-tauri/target/release/betterrgb.exe
src-tauri/target/release/bundle/nsis/betterRGB_0.1.0_x64-setup.exe
src-tauri/target/release/bundle/msi/betterRGB_0.1.0_x64_en-US.msi
```

---

## 🔧 Hardware Compatibility

Tested and optimized for:

- **Motherboards**: ASUS PRIME B760M-A, ASUS ROG Strix, TUF Gaming, and all standard OpenRGB-supported SMBus/I2C ARGB headers.
- **Fans**: 3-pin 5V ARGB daisy-chained fans (up to 240 addressable LEDs).

> **Tip**: If Asus Armoury Crate's `LightingService.exe` locks the motherboard USB HID device, navigate to the **Health & Diagnostics** tab and click **"Pause Asus LightingService"**.

---

## 📄 License

MIT License © 2026 imsud
