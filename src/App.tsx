import React, { useEffect, useState, useRef, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  Type,
  Blocks,
} from "lucide-react";
import { TitleBar } from "./components/TitleBar";
import { SpotlightCard } from "./components/SpotlightCard";
import { RotaryKnob } from "./components/RotaryKnob";

interface RgbColor {
  r: number;
  g: number;
  b: number;
}

interface ZoneInfo {
  id: number;
  name: string;
  zone_type: number;
  leds_min: number;
  leds_max: number;
  leds_count: number;
}

interface DeviceInfo {
  id: number;
  name: string;
  vendor: string;
  description: string;
  led_count: number;
  zones?: ZoneInfo[];
}

interface AppStatePayload {
  connected: boolean;
  active_mode: string;
  brightness: number;
  static_color: RgbColor;
  current_color: RgbColor;
  cpu_usage: number;
  cpu_temp: number | null;
  active_app: string;
  is_coding_detected: boolean;
  is_gaming_detected: boolean;
  devices: DeviceInfo[];
  led_count: number;
  audio_bands: number[];
  audio_sensitivity?: number;
  app_color?: RgbColor;
  debug?: string;
  debug_index?: number;
  debug_paused?: boolean;
  layout?: PanelLayout;
  viz_style?: string;
  marquee_text?: string;
  game_speed?: number;
}

interface PanelLayout {
  lanes: number;
  leds_per_lane: number;
  first_index: number;
  serpentine: boolean;
  bass_at_top: boolean;
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

// Module-level so it isn't rebuilt on every render.
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
  {
    id: "text",
    name: "Scrolling Message",
    description:
      "Marquee text scrolls up the panel in your accent colour. Set the message below.",
    icon: Type,
    color: "rgba(244, 114, 182, 0.16)",
    glow: "#f472b6",
  },
  {
    id: "snake",
    name: "Snake (Auto-Play)",
    description:
      "A self-playing snake game runs up the panel, hunting food and avoiding itself.",
    icon: Gamepad2,
    color: "rgba(34, 197, 94, 0.16)",
    glow: "#22c55e",
  },
  {
    id: "tetris",
    name: "Tetris (Auto-Play)",
    description:
      "A self-playing Tetris demo stacks tetrominoes and clears completed lines.",
    icon: Blocks,
    color: "rgba(56, 189, 248, 0.16)",
    glow: "#38bdf8",
  },
];

// Must match NUM_BANDS in src-tauri/src/audio.rs.
const NUM_BANDS = 24;

const BAND_HUES = Array.from({ length: NUM_BANDS }, (_, i) => {
  const t = i / (NUM_BANDS - 1);
  return `hsl(${(330 - t * 275 + 360) % 360}, 95%, 58%)`;
});

/**
 * Minimal bitmap font mirroring src-tauri/src/font.rs, used only to render the
 * on-screen preview.
 *
 * Text is drawn UPRIGHT: each glyph is 3 pixels wide (one per lane) and N tall.
 * Rows are stored top-down, with bit 2 as the leftmost pixel - matching the Rust
 * source. `glyphRowsBottomUp` converts to panel order and lane order.
 */
const PREVIEW_FONT: Record<string, { rows: number[]; h: number }> = {
  A: { rows: [0b010, 0b101, 0b111, 0b101, 0b101], h: 5 },
  B: { rows: [0b110, 0b101, 0b110, 0b101, 0b110], h: 5 },
  C: { rows: [0b011, 0b100, 0b100, 0b100, 0b011], h: 5 },
  D: { rows: [0b110, 0b101, 0b101, 0b101, 0b110], h: 5 },
  E: { rows: [0b111, 0b100, 0b110, 0b100, 0b111], h: 5 },
  F: { rows: [0b111, 0b100, 0b110, 0b100, 0b100], h: 5 },
  G: { rows: [0b011, 0b100, 0b101, 0b101, 0b011], h: 5 },
  H: { rows: [0b101, 0b101, 0b111, 0b101, 0b101], h: 5 },
  I: { rows: [0b111, 0b010, 0b010, 0b010, 0b111], h: 5 },
  J: { rows: [0b001, 0b001, 0b001, 0b101, 0b111], h: 5 },
  K: { rows: [0b101, 0b101, 0b110, 0b101, 0b101], h: 5 },
  L: { rows: [0b100, 0b100, 0b100, 0b100, 0b111], h: 5 },
  M: { rows: [0b101, 0b111, 0b101, 0b101, 0b101], h: 5 },
  N: { rows: [0b101, 0b111, 0b111, 0b101, 0b101], h: 5 },
  O: { rows: [0b111, 0b101, 0b101, 0b101, 0b111], h: 5 },
  P: { rows: [0b110, 0b101, 0b110, 0b100, 0b100], h: 5 },
  Q: { rows: [0b111, 0b101, 0b101, 0b111, 0b001], h: 5 },
  R: { rows: [0b110, 0b101, 0b110, 0b101, 0b101], h: 5 },
  S: { rows: [0b011, 0b100, 0b010, 0b001, 0b110], h: 5 },
  T: { rows: [0b111, 0b010, 0b010, 0b010, 0b010], h: 5 },
  U: { rows: [0b101, 0b101, 0b101, 0b101, 0b111], h: 5 },
  V: { rows: [0b101, 0b101, 0b101, 0b101, 0b010], h: 5 },
  W: { rows: [0b101, 0b101, 0b111, 0b111, 0b101], h: 5 },
  X: { rows: [0b101, 0b101, 0b010, 0b101, 0b101], h: 5 },
  Y: { rows: [0b101, 0b101, 0b010, 0b010, 0b010], h: 5 },
  Z: { rows: [0b111, 0b001, 0b010, 0b100, 0b111], h: 5 },
  "0": { rows: [0b111, 0b101, 0b101, 0b101, 0b101, 0b101, 0b111], h: 7 },
  "1": { rows: [0b010, 0b110, 0b010, 0b010, 0b010, 0b010, 0b111], h: 7 },
  "2": { rows: [0b111, 0b001, 0b001, 0b111, 0b100, 0b100, 0b111], h: 7 },
  "3": { rows: [0b111, 0b001, 0b001, 0b111, 0b001, 0b001, 0b111], h: 7 },
  "4": { rows: [0b101, 0b101, 0b101, 0b111, 0b001, 0b001, 0b001], h: 7 },
  "5": { rows: [0b111, 0b100, 0b100, 0b111, 0b001, 0b001, 0b111], h: 7 },
  "6": { rows: [0b111, 0b100, 0b100, 0b111, 0b101, 0b101, 0b111], h: 7 },
  "7": { rows: [0b111, 0b001, 0b001, 0b010, 0b010, 0b010, 0b010], h: 7 },
  "8": { rows: [0b111, 0b101, 0b101, 0b111, 0b101, 0b101, 0b111], h: 7 },
  "9": { rows: [0b111, 0b101, 0b101, 0b111, 0b001, 0b001, 0b111], h: 7 },
  " ": { rows: [0, 0, 0], h: 3 },
  "-": { rows: [0b000, 0b111, 0b000], h: 3 },
  _: { rows: [0b000, 0b000, 0b111], h: 3 },
  ".": { rows: [0b000, 0b000, 0b010], h: 3 },
  ",": { rows: [0b000, 0b010, 0b100], h: 3 },
  "!": { rows: [0b010, 0b010, 0b010, 0b000, 0b010], h: 5 },
  "?": { rows: [0b111, 0b001, 0b011, 0b000, 0b010], h: 5 },
  ":": { rows: [0b000, 0b010, 0b000], h: 3 },
  "+": { rows: [0b000, 0b010, 0b111, 0b010, 0b000], h: 5 },
  "=": { rows: [0b000, 0b111, 0b000, 0b111, 0b000], h: 5 },
  "/": { rows: [0b001, 0b001, 0b010, 0b100, 0b100], h: 5 },
  "(": { rows: [0b001, 0b010, 0b100, 0b010, 0b001], h: 5 },
  ")": { rows: [0b100, 0b010, 0b001, 0b010, 0b100], h: 5 },
  "'": { rows: [0b010, 0b010, 0b000], h: 3 },
};

const PREVIEW_GAP = 1;

/** Reverse the three pixel bits, matching `mirror3` in the Rust font. */
function mirror3(row: number): number {
  return ((row & 0b001) << 2) | (row & 0b010) | ((row & 0b100) >> 2);
}

/**
 * Flatten text into panel rows, bottom-up and in lane order.
 * Mirrors `marquee_frame` in engine.rs.
 */
function renderMarqueeRows(text: string): number[] {
  const out: number[] = [];
  for (const ch of text.toUpperCase()) {
    const g = PREVIEW_FONT[ch] ?? { rows: [0, 0, 0], h: 3 };
    // Stored top-down; reverse for panel order, mirror for lane order.
    const bottomUp = g.rows.slice(0, g.h).reverse().map(mirror3);
    out.push(...bottomUp);
    for (let i = 0; i < PREVIEW_GAP; i++) out.push(0);
  }
  return out;
}

/**
 * Build the panel preview grid: `[lane][row]` -> lit or dark.
 *
 * Mirrors `marquee_frame` in engine.rs exactly, including the trailing blank gap
 * and the scroll offset, so the preview animates like the real hardware.
 */
function buildMarqueePreview(
  text: string,
  lanes: number,
  rows: number,
  scroll: number,
): boolean[][] {
  const grid: boolean[][] = Array.from({ length: lanes }, () =>
    new Array(rows).fill(false),
  );

  const template = renderMarqueeRows(text);
  // Blank gap as tall as the panel, so the message clears before repeating.
  for (let i = 0; i < rows; i++) template.push(0);
  if (template.length === 0) return grid;

  // The engine floors the offset, so the display only changes on whole rows.
  const period = template.length;
  const offset = ((Math.floor(scroll) % period) + period) % period;

  for (let row = 0; row < rows; row++) {
    const glyphRow = template[(row + offset) % period];
    for (let lane = 0; lane < lanes && lane < 3; lane++) {
      if ((glyphRow >> lane) & 1) grid[lane][row] = true;
    }
  }
  return grid;
}

/**
 * Animated panel preview for the scrolling-message mode.
 *
 * Advances the scroll at the same px/s rate sent to the backend and floors it to
 * whole rows exactly as `marquee_frame` does, so what you see matches the fan.
 */
const MarqueePreview: React.FC<{
  text: string;
  speed: number;
  lanes: number;
  rows: number;
}> = ({ text, speed, lanes, rows }) => {
  const [scroll, setScroll] = useState(0);

  useEffect(() => {
    let raf = 0;
    const start = performance.now();

    const tick = (now: number) => {
      // Floor to whole rows before storing. The engine only advances on row
      // boundaries, so setting a fractional value would re-render 60x/sec for a
      // display that changes a few times a second. Storing the floored value lets
      // React skip the unchanged updates.
      const rows_scrolled = Math.floor(((now - start) / 1000) * speed);
      setScroll((prev) => (prev === rows_scrolled ? prev : rows_scrolled));
      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // Restart from the beginning when the message or speed changes, so the new
    // text is shown from its first row rather than mid-message.
  }, [speed, text]);

  const grid = buildMarqueePreview(text, lanes, rows, scroll);

  return (
    <div className="flex items-end gap-3">
      <div className="flex gap-1.5 p-2 rounded-lg bg-slate-950/80 border border-white/[0.06] w-fit">
        {grid.map((lane, laneIdx) => (
          <div key={laneIdx} className="flex flex-col-reverse gap-[2px]">
            {lane.map((isLit, rowIdx) => (
              <span
                key={rowIdx}
                className="h-1.5 w-1.5 rounded-full"
                style={{
                  backgroundColor: isLit ? "#f472b6" : "#1e293b",
                }}
              />
            ))}
          </div>
        ))}
      </div>
      <div className="flex flex-col gap-0.5 text-[10px] font-mono text-slate-500">
        <span>{speed} px/s</span>
        <span>row {scroll}</span>
      </div>
    </div>
  );
};

/**
 * Mirror of `game_color` in games.rs, so the preview shows the same colours as
 * the panel. The accent is whatever the user has selected for the snake body.
 */
function gameColor(idx: number, accent: RgbColor): string {
  const mixWhite = (c: RgbColor, amount: number): RgbColor => {
    const f = (v: number) => Math.round(v + (255 - v) * amount);
    return { r: f(c.r), g: f(c.g), b: f(c.b) };
  };
  const rgb = (c: RgbColor) => `rgb(${c.r},${c.g},${c.b})`;

  switch (idx) {
    case 1: // PAL_HEAD
      return rgb(mixWhite(accent, 0.75));
    case 2: // PAL_FOOD
      return "rgb(255,60,60)";
    case 3: // I
      return "rgb(0,220,255)";
    case 4: // O
      return "rgb(255,220,0)";
    case 5: // T
      return "rgb(170,0,255)";
    case 6: // S
      return "rgb(0,220,80)";
    case 7: // Z
      return "rgb(255,40,60)";
    case 8: // J
      return "rgb(60,90,255)";
    case 9: // L
      return "rgb(255,140,0)";
    case 10: // PAL_CLEAR_A
      return "rgb(255,255,255)";
    case 11: // PAL_CLEAR_B
      return "rgb(255,60,60)";
    default: // PAL_BODY
      return rgb(accent);
  }
}

interface GameSnapshot {
  /** Which game produced this frame: "snake" or "tetris". */
  game: string;
  lanes: (number | null)[][];
  cleared: number;
  flashing: boolean;
  clearing_rows: number[];
}

/**
 * Live preview of the Snake / Tetris board.
 *
 * This renders the *actual* engine state, streamed over the `game-frame` event at
 * tick rate. It deliberately does not simulate the games in the browser: a second
 * implementation would drift from the Rust one, and the whole point is to show
 * what the panel is really doing.
 */
const GamePreview: React.FC<{
  lanes: number;
  rows: number;
  accent: RgbColor;
  title: string;
  /** Which game this preview is for; frames from the other game are ignored. */
  game: "snake" | "tetris";
}> = ({ lanes, rows, accent, title, game }) => {
  const [snapshot, setSnapshot] = useState<GameSnapshot | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    listen<GameSnapshot>("game-frame", (event) => {
      if (disposed) return;
      // Drop frames from the other game. Without this the preview keeps showing
      // the previous game's board after a mode switch, and for snake that means a
      // frozen mid-game position that never matches the panel.
      if (event.payload.game !== game) return;
      setSnapshot(event.payload);
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Non-Tauri (browser dev) fallback: nothing to show.
      });

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, [game]);

  // Belt and braces: also ignore a stored snapshot from the other game, so a mode
  // switch can never show the wrong board even for a single frame.
  const current = snapshot && snapshot.game === game ? snapshot : null;

  // Fall back to a blank grid until the first frame arrives, so the preview keeps
  // its shape instead of collapsing and making the panel jump.
  const grid: (number | null)[][] =
    current?.lanes ??
    Array.from({ length: lanes }, () => new Array(rows).fill(null));

  return (
    <div className="flex items-start gap-3">
      <div className="flex gap-1.5 p-2 rounded-lg bg-slate-950/80 border border-white/[0.06] w-fit">
        {grid.map((lane, laneIdx) => (
          <div key={laneIdx} className="flex flex-col-reverse gap-[2px]">
            {lane.map((idx, rowIdx) => (
              <span
                key={rowIdx}
                className="h-1.5 w-1.5 rounded-full"
                style={{
                  backgroundColor:
                    idx === null ? "#1e293b" : gameColor(idx, accent),
                }}
              />
            ))}
          </div>
        ))}
      </div>
      <div className="flex flex-col gap-0.5 text-[10px] font-mono text-slate-500">
        <span className="text-slate-400">{title}</span>
        <span>
          {lanes} × {rows}
        </span>
        {!current && <span className="text-amber-400">waiting…</span>}
        {current?.flashing && (
          <span className="text-emerald-400 font-semibold">
            CLEAR {(current.clearing_rows ?? []).join(", ")}
          </span>
        )}
        {current !== null && current.cleared > 0 && (
          <span>lines {current.cleared}</span>
        )}
      </div>
    </div>
  );
};

/** Diagnostic patterns exposed on the Debug page. */
const DEBUG_PATTERNS: {
  id: string;
  name: string;
  what: string;
  how: string;
}[] = [
  {
    id: "off",
    name: "Off",
    what: "Normal lighting output.",
    how: "Use this to leave test mode.",
  },
  {
    id: "all_white",
    name: "All White",
    what: "Lights every LED solid white.",
    how: "Count how many LEDs actually light up. Any dark gaps are dead, unsupported, or wired past the end of the strip.",
  },
  {
    id: "count",
    name: "Count Marker",
    what: "Repeats 10 red / 10 green / 10 blue / 10 white.",
    how: "Count the first red run, then multiply by the number of full colour groups. Sum the partial final group for the exact total.",
  },
  {
    id: "single",
    name: "Single LED",
    what: "Lights exactly one LED at the index you choose.",
    how: "Step through indices one at a time. If extra LEDs light at the same moment, they are wired in PARALLEL to that channel. Only one lighting means TRUE daisy-chain.",
  },
  {
    id: "snake",
    name: "Snake (Index Tracer)",
    what: "A pulse walks the strip one LED at a time, with a dim ruler every 10th LED.",
    how: "Use the LED number box to read or jump to a position. Type an index and the snake pins there, so you can confirm exactly which physical LED that index maps to.",
  },
  {
    id: "tens",
    name: "Ruler (every 10th)",
    what: "Lights every 10th LED bright, every 5th dim.",
    how: "A numbered ruler that survives losing your place while counting a long strip.",
  },
  {
    id: "halves",
    name: "Halves",
    what: "First half red, second half blue.",
    how: "If the two halves look reversed or interleaved on your hardware, the strip is mirrored between channels.",
  },
];

/** 32-band audio meter. Memoized; bars snap straight to each reading, no tweening. */
const AudioBars = memo(function AudioBars({ bands }: { bands: number[] }) {
  return (
    <div className="flex h-11 items-end gap-[2px] px-0.5">
      {bands.map((band, idx) => (
        <div
          key={idx}
          className="flex-1 rounded-[1px] bg-slate-800/60 relative overflow-hidden"
          style={{ height: "100%" }}
          title={`Band ${idx + 1}: ${Math.round(band * 100)}%`}
        >
          <div
            className="w-full absolute bottom-0 rounded-[1px] origin-bottom"
            style={{
              height: "100%",
              backgroundColor: BAND_HUES[idx],
              transform: `scaleY(${Math.min(1, Math.max(0.04, band))})`,
            }}
          />
        </div>
      ))}
    </div>
  );
});

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
    "lighting" | "diagnostics" | "debug" | "profiles" | "settings"
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
          cpu_temp: null,
          active_app: "Desktop",
          is_coding_detected: false,
          is_gaming_detected: false,
          devices: [],
          led_count: parsed.led_count || 40,
          audio_bands: new Array(NUM_BANDS).fill(0),
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
      cpu_temp: null,
      active_app: "Desktop",
      is_coding_detected: false,
      is_gaming_detected: false,
      devices: [],
      led_count: 40,
      audio_bands: new Array(NUM_BANDS).fill(0),
      audio_sensitivity: 1.0,
    };
  });

  const [notification, setNotification] = useState<string | null>(null);
  const [startingOpenRgb, setStartingOpenRgb] = useState(false);

  // Debug page state
  const [debugIndex, setDebugIndex] = useState(0);
  const [debugPaused, setDebugPaused] = useState(false);
  // Prevents the poll from overwriting the index box while it is being typed in.
  const indexInputFocused = useRef(false);
  // Marquee message for Text mode.
  const [marqueeText, setMarqueeText] = useState("BETTERRGB");
  const [marqueeSpeed, setMarqueeSpeed] = useState(14);
  const marqueeInputFocused = useRef(false);
  // Play-speed multiplier for the Snake and Tetris demos.
  const [gameSpeed, setGameSpeed] = useState(1.0);
  // Physical grid arrangement. Defaults match the confirmed fan: 3 lanes of 13,
  // index 0 unwired, every lane running bottom to top.
  const [laneConfig, setLaneConfig] = useState<PanelLayout>({
    lanes: 3,
    leds_per_lane: 13,
    first_index: 1,
    serpentine: false,
    bass_at_top: false,
  });

  // Audio bands arrive as a pushed event at engine tick rate (~30/s). Polling
  // could only deliver ~4/s, which made the meter look like a 1-2 Hz animation.
  const [liveBands, setLiveBands] = useState<number[]>(() =>
    new Array(NUM_BANDS).fill(0),
  );

  // Mirrors `state` for read access inside the polling loop without adding a dep.
  const prevStateRef = useRef<AppStatePayload | null>(null);

  // Autostart setting state
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [loadingAutostart, setLoadingAutostart] = useState(false);

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

  // Push-based audio bands from the engine. Runs at tick rate so the meter
  // tracks the audio directly instead of being limited by the poll interval.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<number[]>("audio-bands", (event) => {
      setLiveBands(event.payload);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // Not running inside Tauri
      });
    return () => unlisten?.();
  }, []);

  // Adaptive state polling. Uses the Tauri window API rather than document.hidden,
  // which stays false when the window is hidden to tray. Backs off to a slow poll
  // when hidden and self-schedules to avoid overlapping requests.
  useEffect(() => {
    let isMounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let active = true;
    const POLL_ACTIVE_MS = 250; // visible+centered: matches the original cadence
    const POLL_IDLE_MS = 500; // visible but unfocused
    const POLL_HIDDEN_MS = 15000; // tray-only: near-zero wakeups

    const fetchState = async () => {
      try {
        const res = await invoke<AppStatePayload>("get_state");
        if (!isMounted) return;

        // Bail out before touching state when nothing changed, to avoid a re-render
        // that would restart every CSS transition in the tree. The previous payload
        // is tracked in a ref so this comparison stays outside the state updater.
        // Audio bands are excluded - they arrive via the push event above.
        // Debug fields ARE included, otherwise a travelling snake would be skipped
        // and the index readout would freeze.
        const prev = prevStateRef.current;

        const unchanged =
          prev !== null &&
          prev.connected === res.connected &&
          prev.active_mode === res.active_mode &&
          prev.current_color.r === res.current_color.r &&
          prev.current_color.g === res.current_color.g &&
          prev.current_color.b === res.current_color.b &&
          prev.cpu_temp === res.cpu_temp &&
          prev.cpu_usage === res.cpu_usage &&
          prev.active_app === res.active_app &&
          prev.is_coding_detected === res.is_coding_detected &&
          prev.is_gaming_detected === res.is_gaming_detected &&
          prev.devices.length === res.devices.length &&
          prev.debug === res.debug &&
          prev.debug_index === res.debug_index &&
          prev.debug_paused === res.debug_paused;

        if (unchanged) return;

        prevStateRef.current = res;
        setState(res);

        // Keep the snake's position control in sync with the engine. Skipped
        // while the user is typing so the field doesn't fight their input.
        if (res.debug === "snake" && !res.debug_paused) {
          if (!indexInputFocused.current) {
            setDebugIndex(res.debug_index ?? 0);
          }
        }
        if (res.debug_paused !== undefined) {
          setDebugPaused(res.debug_paused);
        }
        if (res.layout) {
          setLaneConfig(res.layout);
        }
        if (res.marquee_text !== undefined && !marqueeInputFocused.current) {
          setMarqueeText(res.marquee_text);
        }
        if (res.game_speed !== undefined) {
          setGameSpeed(res.game_speed);
        }
      } catch {
        // Dev server fallback
      }
    };

    const loop = async () => {
      if (!isMounted) return;

      // Determine true visibility/minimised state from the OS window, not the DOM.
      let focused = true;
      let visible = true;
      try {
        const win = getCurrentWindow();
        focused = await win.isFocused();
        visible = await win.isVisible();
      } catch {
        // Non-Tauri (browser dev) fallback
        visible = typeof document === "undefined" || !document.hidden;
      }
      active = visible;

      if (visible) {
        await fetchState();
      }

      const delay = !visible
        ? POLL_HIDDEN_MS
        : focused && active
          ? POLL_ACTIVE_MS
          : POLL_IDLE_MS;

      if (isMounted) timer = setTimeout(loop, delay);
    };

    loop();

    // Coming back to the window or un-minimising should refresh immediately
    // rather than waiting for the next slow idle tick.
    const wake = () => {
      if (!isMounted) return;
      clearTimeout(timer);
      loop();
    };
    document.addEventListener("visibilitychange", wake);

    return () => {
      isMounted = false;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", wake);
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

  const handleSetDebugPattern = async (pattern: string) => {
    try {
      await invoke("set_debug_pattern", { pattern });
      // Snake resumes travelling whenever it is (re)started.
      if (pattern === "snake") {
        await invoke("set_debug_paused", { paused: false });
      }
    } catch (e: any) {
      showToast(`Debug error: ${e}`);
    }
  };

  // Jump to an exact index. Pins the snake so the LED you asked for stays lit.
  const handleSetDebugIndex = async (index: number) => {
    setDebugIndex(index);
    try {
      await invoke("set_debug_index", { index, pause: true });
    } catch (e: any) {
      showToast(`Debug error: ${e}`);
    }
  };

  const handleToggleDebugPause = async () => {
    const next = !debugPaused;
    setDebugPaused(next);
    try {
      await invoke("set_debug_paused", { paused: next });
    } catch (e: any) {
      showToast(`Debug error: ${e}`);
    }
  };

  const handleLaneChange = async (next: Partial<PanelLayout>) => {
    const merged = { ...laneConfig, ...next };
    setLaneConfig(merged);
    try {
      await invoke("set_lane_layout", {
        lanes: merged.lanes,
        ledsPerLane: merged.leds_per_lane,
        firstIndex: merged.first_index,
        serpentine: merged.serpentine,
        bassAtTop: merged.bass_at_top,
      });
      // Keep the total LED count in step with the grid, so the effect always
      // covers the real hardware and unused slots stay dark.
      const total = merged.first_index + merged.lanes * merged.leds_per_lane;
      if (total !== state.led_count) {
        await invoke("set_led_count", { count: total });
      }
    } catch (e: any) {
      showToast(`Layout error: ${e}`);
    }
  };

  const handleVizStyle = async (style: string) => {
    try {
      await invoke("set_viz_style", { style });
    } catch (e: any) {
      showToast(`Visualizer error: ${e}`);
    }
  };

  const handleMarqueeText = (text: string) => {
    // Local update keeps typing responsive; the backend filters unsupported
    // characters and is the source of truth on the next poll.
    setMarqueeText(text);
    invoke("set_marquee_text", { text }).catch(() => {});
  };

  const handleMarqueeSpeed = (speed: number) => {
    setMarqueeSpeed(speed);
    invoke("set_marquee_speed", { speed }).catch(() => {});
  };

  const handleGameSpeed = (speed: number) => {
    setGameSpeed(speed);
    invoke("set_game_speed", { speed }).catch(() => {});
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

  // Audio and CPU telemetry are only surfaced for the modes that actually
  // consume them. In Smart/auto they stay hidden by default.
  const audioActive = state.active_mode === "visualizer";
  const telemetryActive =
    state.active_mode === "thermal" || state.active_mode === "gaming";

  // Game console: speed control plus a live view of the real board. Rendered in
  // both the Lighting tab (as the counterpart to the visualizer banner) and the
  // Debug tab, so it is reachable wherever the mode was switched on.
  const isGameMode =
    state.active_mode === "snake" || state.active_mode === "tetris";
  const gamePanel = isGameMode ? (
    <div className="glass-panel rounded-2xl p-5 flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-1.5">
          {state.active_mode === "tetris" ? (
            <Blocks className="h-3.5 w-3.5 text-sky-400" />
          ) : (
            <Gamepad2 className="h-3.5 w-3.5 text-emerald-400" />
          )}
          {state.active_mode === "tetris" ? "Tetris Demo" : "Snake Demo"}
        </span>
        <span className="text-[10px] px-2 py-0.5 rounded-full bg-white/[0.06] border border-white/[0.1] text-slate-400 font-semibold">
          Auto-play
        </span>
      </div>

      <div className="flex items-center gap-3">
        <span className="text-[10px] text-slate-400 shrink-0">Speed</span>
        <input
          type="range"
          min={0.25}
          max={4}
          step={0.25}
          value={gameSpeed}
          onChange={(e) => handleGameSpeed(parseFloat(e.target.value) || 1)}
          className="flex-1 cursor-pointer"
        />
        <span className="text-[11px] font-mono text-slate-300 w-14 text-right">
          {gameSpeed.toFixed(2)}×
        </span>
      </div>

      <div className="flex gap-1.5">
        {[0.5, 1, 2, 3].map((v) => (
          <button
            key={v}
            onClick={() => handleGameSpeed(v)}
            className={`flex-1 px-2 py-1 rounded-lg border text-[10px] font-semibold transition-all cursor-pointer ${
              gameSpeed === v
                ? "border-cyan-400 bg-cyan-500/20 text-cyan-300"
                : "border-white/[0.1] bg-white/[0.04] text-slate-400 hover:bg-white/[0.08]"
            }`}
          >
            {v}×
          </button>
        ))}
      </div>

      {/* Live board streamed from the engine, not simulated here. */}
      <div className="flex flex-col gap-1.5">
        <span className="text-[10px] text-slate-400">
          Live preview ({laneConfig.lanes} lanes × {laneConfig.leds_per_lane}{" "}
          rows)
        </span>
        <GamePreview
          key={state.active_mode}
          game={state.active_mode === "tetris" ? "tetris" : "snake"}
          lanes={laneConfig.lanes}
          rows={laneConfig.leds_per_lane}
          accent={state.static_color}
          title={state.active_mode === "tetris" ? "Stacking" : "Hunting food"}
        />
      </div>

      <p className="text-[11px] text-slate-500 leading-relaxed">
        {state.active_mode === "tetris"
          ? "Pieces spawn at the top, rotate to the chosen orientation, then fall one row at a time. A completed line flashes before it clears, and the stack pauses while it does."
          : "The snake hunts for food and avoids itself, restarting automatically when it dies. Its head is the bright tip."}
      </p>
    </div>
  ) : null;

  return (
    <div className="relative flex h-screen w-screen flex-col bg-[#07090e] text-slate-100 overflow-hidden font-sans">
      {/* Aurora ambient glow. Plain radial gradients instead of large blur-radius
          layers, which are expensive to rasterize and were re-rasterized on every colour change. */}
      <div
        className="pointer-events-none fixed inset-0 z-0"
        style={{
          opacity: 0.25,
          background: isOff
            ? "none"
            : `radial-gradient(600px circle at 25% 12%, ${currentColorHex}59, transparent 70%), radial-gradient(600px circle at 75% 88%, ${currentColorHex}38, transparent 70%)`,
        }}
      />

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

      {/* Toast Notification (CSS-animated; no framer-motion runtime needed) */}
      {notification && (
        <div className="toast-anim absolute top-14 right-6 z-50 flex items-center gap-2.5 rounded-xl border border-cyan-400/30 bg-slate-900 px-4 py-2.5 text-xs font-medium shadow-2xl">
          <Zap className="h-4 w-4 text-cyan-400" />
          <span className="text-slate-200">{notification}</span>
        </div>
      )}

      {/* App Workspace Body */}
      <div className="flex-1 flex overflow-hidden relative z-10">
        {/* Left Navigation & Telemetry Sidebar */}
        <aside className="w-80 border-r border-white/[0.08] bg-[#090d16] flex flex-col justify-between p-4 shrink-0 overflow-y-auto">
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
                onClick={() => setActiveTab("debug")}
                className={`flex-1 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                  activeTab === "debug"
                    ? "bg-amber-500/20 text-amber-300 shadow-[0_0_12px_rgba(245,158,11,0.2)] border border-amber-500/30"
                    : "text-slate-400 hover:text-white"
                }`}
              >
                Debug
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
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/70 p-3.5">
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

            {/* Live Telemetry: CPU Temperature. Shows N/A when the machine exposes
                no CPU thermal sensor, rather than inventing a plausible number. */}
            <div className="rounded-xl border border-white/[0.06] bg-slate-950/70 p-3.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                  <Thermometer className="h-3.5 w-3.5 text-amber-400" />
                  CPU Temperature
                </span>
                <span
                  className={`text-lg font-bold tabular-nums ${
                    state.cpu_temp === null
                      ? "text-slate-500"
                      : state.cpu_temp > 75
                        ? "text-red-400"
                        : state.cpu_temp > 60
                          ? "text-amber-400"
                          : "text-emerald-400"
                  }`}
                  title={
                    state.cpu_temp === null
                      ? "No CPU temperature sensor is available on this system"
                      : undefined
                  }
                >
                  {state.cpu_temp === null
                    ? "N/A"
                    : `${Math.round(state.cpu_temp)}°C`}
                </span>
              </div>
            </div>

            {/* Live Telemetry: CPU Utilization. Only meaningful when hardware
                temperature is driving the lighting, so it is hidden otherwise. */}
            {telemetryActive && (
              <div className="rounded-xl border border-white/[0.06] bg-slate-950/70 p-3.5">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                    <Cpu className="h-3.5 w-3.5 text-cyan-400" />
                    CPU Utilization
                  </span>
                  <span className="text-lg font-bold tabular-nums text-cyan-300">
                    {Math.round(state.cpu_usage)}%
                  </span>
                </div>
              </div>
            )}

            {/* WASAPI Audio Visualizer. Only shown for modes that consume audio,
                so the meter isn't occupying space (or implying activity) when
                the capture stream is shut off. */}
            {audioActive && (
              <div className="rounded-xl border border-white/[0.06] bg-slate-950/70 p-3.5 flex flex-col gap-2.5">
                <div className="flex items-center justify-between text-[11px] font-medium text-slate-400">
                  <span className="flex items-center gap-1.5">
                    <Volume2 className="h-3.5 w-3.5 text-violet-400" />
                    WASAPI Audio Equalizer
                  </span>
                  <span className="text-[10px] font-semibold text-violet-400/90 bg-violet-500/10 px-1.5 py-0.5 rounded border border-violet-500/20">
                    {NUM_BANDS} Bands
                  </span>
                </div>
                <AudioBars bands={liveBands} />

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
            )}
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
                                <span className="h-1.5 w-1.5 rounded-full bg-cyan-400" />
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
                <div className="glass-panel rounded-2xl p-5 border border-violet-500/25 bg-gradient-to-r from-violet-950/40 via-slate-900/90 to-purple-950/30 flex items-center justify-between shadow-[0_0_30px_rgba(168,85,247,0.08)]">
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

              {/* Game demo console, the counterpart to the visualizer banner. */}
              {gamePanel}

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
                      min="1"
                      max="480"
                      step="1"
                      value={state.led_count}
                      onChange={(e) =>
                        handleLedCountChange(parseInt(e.target.value) || 1)
                      }
                      className="w-full cursor-pointer"
                    />
                    <div className="flex items-center gap-1.5">
                      <input
                        type="number"
                        min={1}
                        max={480}
                        value={state.led_count}
                        onChange={(e) =>
                          handleLedCountChange(parseInt(e.target.value) || 1)
                        }
                        className="w-16 px-2 py-0.5 rounded bg-slate-950 border border-white/[0.1] text-[11px] font-mono text-cyan-300 text-center"
                      />
                      {[36, 39, 72, 120, 180].map((count) => (
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
                      className="rounded-xl bg-slate-950/60 p-3 border border-white/[0.06] flex flex-col gap-2"
                    >
                      <div className="flex items-center justify-between">
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

                      {/* Per-zone LED counts. Use these real numbers to set the
                          header fill length so the whole colour ramp is used. */}
                      {dev.zones && dev.zones.length > 0 && (
                        <div className="flex flex-col gap-1 pt-1 border-t border-white/[0.06]">
                          {dev.zones.map((z) => (
                            <div
                              key={z.id}
                              className="flex items-center justify-between text-[11px]"
                            >
                              <span className="text-slate-400 truncate">
                                Zone {z.id}: {z.name || "(unnamed)"}
                              </span>
                              <span className="font-mono text-cyan-300 shrink-0 ml-2">
                                {z.leds_count} LEDs
                                <span className="text-slate-500">
                                  {" "}
                                  (min {z.leds_min} / max {z.leds_max})
                                </span>
                              </span>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {activeTab === "debug" && (
            <div className="flex flex-col gap-5">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <h2 className="text-base font-bold text-white flex items-center gap-2">
                    <Wrench className="h-4 w-4 text-amber-400" />
                    LED Debug & Wiring Test
                  </h2>
                  <p className="text-xs text-slate-400 mt-1">
                    Drive the hardware directly to measure the real LED count
                    and work out how the channels are wired.
                  </p>
                </div>
                <button
                  onClick={() => handleSetDebugPattern("off")}
                  className={`px-4 py-2 rounded-xl border text-xs font-bold transition-all cursor-pointer shrink-0 ${
                    (state.debug ?? "off") !== "off"
                      ? "border-red-500/50 bg-red-500/20 text-red-300 hover:bg-red-500/30"
                      : "border-white/[0.08] bg-white/[0.04] text-slate-400"
                  }`}
                >
                  {(state.debug ?? "off") !== "off"
                    ? "Stop Test"
                    : "Test Inactive"}
                </button>
              </div>

              {/* Live test status */}
              {(state.debug ?? "off") !== "off" && (
                <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 flex items-center gap-3">
                  <span className="h-2 w-2 rounded-full bg-amber-400 shrink-0" />
                  <span className="text-xs text-amber-200">
                    Test pattern active:{" "}
                    <span className="font-bold">
                      {DEBUG_PATTERNS.find((p) => p.id === state.debug)?.name ??
                        state.debug}
                    </span>
                    {((state.debug ?? "off") === "single" ||
                      (state.debug ?? "off") === "snake") && (
                      <>
                        {" "}
                        — LED index{" "}
                        <span className="font-mono font-bold">
                          {debugIndex}
                        </span>
                        {(state.debug ?? "off") === "snake" && (
                          <span className="text-amber-300/80">
                            {debugPaused ? " (paused)" : " (travelling)"}
                          </span>
                        )}
                      </>
                    )}
                    . Normal lighting is suspended.
                  </span>
                </div>
              )}

              {/* LED count control, scoped to testing so it can be changed
                  without leaving the Debug page. */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-4">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-1.5">
                    <Layers className="h-3.5 w-3.5 text-cyan-400" />
                    Total LED Count
                  </span>
                  <span className="text-sm font-bold font-mono text-cyan-300">
                    {state.led_count}
                  </span>
                </div>

                <input
                  type="range"
                  min={1}
                  max={480}
                  step={1}
                  value={state.led_count}
                  onChange={(e) =>
                    handleLedCountChange(parseInt(e.target.value) || 1)
                  }
                  className="w-full cursor-pointer"
                />

                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    min={1}
                    max={480}
                    value={state.led_count}
                    onChange={(e) =>
                      handleLedCountChange(parseInt(e.target.value) || 1)
                    }
                    className="w-20 px-2 py-1 rounded-lg bg-slate-950 border border-white/[0.1] text-xs font-mono text-cyan-300 text-center"
                  />
                  <button
                    onClick={() => handleLedCountChange(state.led_count - 1)}
                    className="h-8 w-8 rounded-lg border border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08] font-bold cursor-pointer"
                  >
                    −
                  </button>
                  <button
                    onClick={() => handleLedCountChange(state.led_count + 1)}
                    className="h-8 w-8 rounded-lg border border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08] font-bold cursor-pointer"
                  >
                    +
                  </button>
                  <div className="flex items-center gap-1.5 flex-1 justify-end">
                    {[13, 39, 40, 78, 120].map((count) => (
                      <button
                        key={count}
                        onClick={() => handleLedCountChange(count)}
                        className={`px-2 py-1 text-[10px] rounded border font-medium transition-all cursor-pointer ${
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

                <p className="text-[11px] text-slate-500 leading-relaxed">
                  Matches the header fill length on the Lighting tab. Raise it
                  until the pattern covers your whole strip; lower it if the
                  pattern repeats before the strip ends. Presets cover the
                  common sizes for your 13-per-line fans.
                </p>
              </div>

              {/* LED grid. Effects are rendered per (row, column) because the
                  strip is physically a grid, not a line. */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-4">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-1.5">
                    <Layers className="h-3.5 w-3.5 text-emerald-400" />
                    LED Grid
                  </span>
                  <span className="text-[11px] text-slate-500">
                    {laneConfig.lanes} lanes × {laneConfig.leds_per_lane} rows
                  </span>
                </div>

                <div className="grid grid-cols-3 gap-3">
                  <div className="flex flex-col gap-1">
                    <label className="text-[10px] text-slate-400">Lanes</label>
                    <input
                      type="number"
                      min={1}
                      max={16}
                      value={laneConfig.lanes}
                      onChange={(e) =>
                        handleLaneChange({
                          lanes: Math.max(1, parseInt(e.target.value) || 1),
                        })
                      }
                      className="px-2 py-1 rounded-lg bg-slate-950 border border-white/[0.1] text-xs font-mono text-emerald-300 text-center"
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-[10px] text-slate-400">
                      LEDs / lane
                    </label>
                    <input
                      type="number"
                      min={1}
                      max={120}
                      value={laneConfig.leds_per_lane}
                      onChange={(e) =>
                        handleLaneChange({
                          leds_per_lane: Math.max(
                            1,
                            parseInt(e.target.value) || 1,
                          ),
                        })
                      }
                      className="px-2 py-1 rounded-lg bg-slate-950 border border-white/[0.1] text-xs font-mono text-emerald-300 text-center"
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className="text-[10px] text-slate-400">
                      First index
                    </label>
                    <input
                      type="number"
                      min={0}
                      max={63}
                      value={laneConfig.first_index}
                      onChange={(e) =>
                        handleLaneChange({
                          first_index: Math.max(
                            0,
                            parseInt(e.target.value) || 0,
                          ),
                        })
                      }
                      className="px-2 py-1 rounded-lg bg-slate-950 border border-white/[0.1] text-xs font-mono text-emerald-300 text-center"
                    />
                  </div>
                </div>

                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] text-slate-400">
                    Lane direction
                  </label>
                  <div className="grid grid-cols-2 gap-1.5">
                    <button
                      onClick={() => handleLaneChange({ serpentine: false })}
                      className={`py-1.5 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                        !laneConfig.serpentine
                          ? "border-emerald-400 bg-emerald-500/20 text-emerald-300"
                          : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                      }`}
                    >
                      All bottom → top
                    </button>
                    <button
                      onClick={() => handleLaneChange({ serpentine: true })}
                      className={`py-1.5 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                        laneConfig.serpentine
                          ? "border-emerald-400 bg-emerald-500/20 text-emerald-300"
                          : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                      }`}
                    >
                      Serpentine
                    </button>
                  </div>
                </div>

                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] text-slate-400">
                    Spectrum orientation
                  </label>
                  <div className="grid grid-cols-2 gap-1.5">
                    <button
                      onClick={() => handleLaneChange({ bass_at_top: false })}
                      className={`py-1.5 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                        !laneConfig.bass_at_top
                          ? "border-violet-400 bg-violet-500/20 text-violet-300"
                          : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                      }`}
                    >
                      Bass at bottom
                    </button>
                    <button
                      onClick={() => handleLaneChange({ bass_at_top: true })}
                      className={`py-1.5 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                        laneConfig.bass_at_top
                          ? "border-violet-400 bg-violet-500/20 text-violet-300"
                          : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                      }`}
                    >
                      Bass at top
                    </button>
                  </div>
                </div>

                <p className="text-[11px] text-slate-500 leading-relaxed">
                  Matches your measured wiring: 3 lanes of 13, index 0 unwired,
                  every lane running bottom to top. Effects are rendered on a
                  grid, so a row spans all three lanes and stays continuous
                  across the fan. Changing the grid also updates the total LED
                  count to match.
                </p>
              </div>

              {/* Visualizer style picker */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-3">
                <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-1.5">
                  <Volume2 className="h-3.5 w-3.5 text-violet-400" />
                  Visualizer Style
                </span>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    {
                      id: "columns",
                      label: "Bars",
                      note: "One vertical bar per lane: bass, mid, treble, with peak markers.",
                    },
                    {
                      id: "rows",
                      label: "Rows",
                      note: "Each frequency band is one full-width horizontal line.",
                    },
                    {
                      id: "bloom",
                      label: "Bloom",
                      note: "Light blooms outward from the centre row.",
                    },
                  ].map((s) => (
                    <button
                      key={s.id}
                      onClick={() => handleVizStyle(s.id)}
                      title={s.note}
                      className={`py-2 rounded-lg border text-xs font-semibold transition-all cursor-pointer ${
                        state.viz_style === s.id
                          ? "border-violet-400 bg-violet-500/20 text-violet-300"
                          : "border-white/[0.06] bg-white/[0.02] text-slate-400 hover:text-white hover:bg-white/[0.06]"
                      }`}
                    >
                      {s.label}
                    </button>
                  ))}
                </div>
                <p className="text-[11px] text-slate-500 leading-relaxed">
                  {
                    {
                      columns:
                        "Bars: each lane is an independent level meter. Peak markers rise instantly and fall slowly, showing recent maxima.",
                      rows: "Rows: all 13 rows span every lane, so each band reads as one continuous line across the fan.",
                      bloom:
                        "Bloom: brightness expands from the centre row outward, strongest at the middle.",
                    }[state.viz_style ?? "columns"]
                  }
                </p>
              </div>

              {/* Marquee message, shown when Text mode is selected. */}
              {(state.active_mode === "text" || activeTab === "debug") && (
                <div className="glass-panel rounded-2xl p-5 flex flex-col gap-4">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-1.5">
                      <Type className="h-3.5 w-3.5 text-pink-400" />
                      Scrolling Message
                    </span>
                    <button
                      onClick={() => {
                        handleSetMode("text");
                        showToast("Scrolling message mode active");
                      }}
                      className={`px-3 py-1 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                        state.active_mode === "text"
                          ? "border-pink-400 bg-pink-500/20 text-pink-300"
                          : "border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08]"
                      }`}
                    >
                      {state.active_mode === "text" ? "Running" : "Run"}
                    </button>
                  </div>

                  <input
                    type="text"
                    value={marqueeText}
                    maxLength={120}
                    placeholder="BETTERRGB"
                    onFocus={() => {
                      marqueeInputFocused.current = true;
                    }}
                    onBlur={() => {
                      marqueeInputFocused.current = false;
                    }}
                    onChange={(e) => handleMarqueeText(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-slate-950 border border-white/[0.1] text-sm text-pink-200 placeholder:text-slate-600 focus:border-pink-500/50 focus:outline-none"
                  />

                  <div className="flex items-center gap-3">
                    <span className="text-[10px] text-slate-400 shrink-0">
                      Speed
                    </span>
                    <input
                      type="range"
                      min={1}
                      max={60}
                      step={1}
                      value={marqueeSpeed}
                      onChange={(e) =>
                        handleMarqueeSpeed(parseInt(e.target.value) || 14)
                      }
                      className="flex-1 cursor-pointer"
                    />
                    <span className="text-[11px] font-mono text-slate-300 w-14 text-right">
                      {marqueeSpeed} px/s
                    </span>
                  </div>

                  {/* Live preview, animating at the same rate as the hardware. */}
                  <div className="flex flex-col gap-1.5">
                    <span className="text-[10px] text-slate-400">
                      Preview (3 lanes × {laneConfig.leds_per_lane} rows)
                    </span>
                    <MarqueePreview
                      text={marqueeText}
                      speed={marqueeSpeed}
                      lanes={laneConfig.lanes}
                      rows={laneConfig.leds_per_lane}
                    />
                  </div>

                  <p className="text-[11px] text-slate-500 leading-relaxed">
                    Text scrolls upward one row at a time, animated here at the
                    same rate as the panel. Letters are upright and read
                    vertically. Supported: A–Z, 0–9 and common punctuation.
                  </p>
                </div>
              )}

              {/* Game demos: speed control and a live preview of the real board. */}
              {gamePanel}

              {/* Index control: used by Single (pinpoint) and Snake (trace) */}
              {((state.debug ?? "off") === "single" ||
                (state.debug ?? "off") === "snake") && (
                <div className="glass-panel rounded-2xl p-5 flex flex-col gap-4">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold uppercase tracking-wider text-slate-300">
                      {(state.debug ?? "off") === "snake"
                        ? "Snake Position"
                        : "Target LED Index"}
                    </span>
                    {(state.debug ?? "off") === "snake" && (
                      <button
                        onClick={handleToggleDebugPause}
                        className={`px-3 py-1 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer ${
                          debugPaused
                            ? "border-emerald-400/50 bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30"
                            : "border-amber-400/50 bg-amber-500/20 text-amber-300 hover:bg-amber-500/30"
                        }`}
                      >
                        {debugPaused ? "Resume" : "Pause"}
                      </button>
                    )}
                  </div>

                  <div className="flex items-center gap-3">
                    <button
                      onClick={() =>
                        handleSetDebugIndex(Math.max(0, debugIndex - 1))
                      }
                      className="h-9 w-9 rounded-lg border border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08] font-bold cursor-pointer"
                    >
                      −
                    </button>
                    <input
                      type="number"
                      min={0}
                      max={999}
                      value={debugIndex}
                      onFocus={() => {
                        indexInputFocused.current = true;
                      }}
                      onBlur={() => {
                        indexInputFocused.current = false;
                      }}
                      onChange={(e) =>
                        handleSetDebugIndex(
                          Math.max(0, parseInt(e.target.value) || 0),
                        )
                      }
                      className="w-24 px-3 py-1.5 rounded-lg bg-slate-950 border border-amber-500/40 text-sm font-mono text-amber-300 text-center"
                    />
                    <button
                      onClick={() => handleSetDebugIndex(debugIndex + 1)}
                      className="h-9 w-9 rounded-lg border border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08] font-bold cursor-pointer"
                    >
                      +
                    </button>
                    <input
                      type="range"
                      min={0}
                      max={Math.max(1, state.led_count - 1)}
                      step={1}
                      value={debugIndex}
                      onChange={(e) =>
                        handleSetDebugIndex(parseInt(e.target.value) || 0)
                      }
                      className="flex-1 cursor-pointer"
                    />
                  </div>

                  {/* Position readout with ruler marks every 10 LEDs */}
                  <div className="flex flex-col gap-1.5">
                    <div className="relative h-6 rounded-lg bg-slate-950/80 border border-white/[0.06] overflow-hidden">
                      {Array.from({ length: 11 }, (_, k) => {
                        const pct = k * 10;
                        return (
                          <div
                            key={k}
                            className="absolute top-0 bottom-0 w-px bg-white/15"
                            style={{ left: `${pct}%` }}
                          />
                        );
                      })}
                      <div
                        className="absolute top-0 bottom-0 w-0.5 bg-amber-400"
                        style={{
                          left: `${
                            state.led_count > 1
                              ? (debugIndex / (state.led_count - 1)) * 100
                              : 0
                          }%`,
                        }}
                      />
                    </div>
                    <div className="flex justify-between text-[9px] font-mono text-slate-500">
                      {[0, 25, 50, 75, 100].map((pct) => (
                        <span key={pct}>
                          {Math.round(((state.led_count - 1) * pct) / 100)}
                        </span>
                      ))}
                    </div>
                  </div>

                  <p className="text-[11px] text-slate-500 leading-relaxed">
                    {(state.debug ?? "off") === "snake"
                      ? "The pulse travels one LED at a time. Type an index to pin it on a specific LED, then read which physical LED lights up. Pause keeps it still while you look."
                      : "Step one index at a time and count how many LEDs light up at each step. One LED per step = true daisy-chain. Several LEDs per step = they are wired in parallel to one channel."}
                  </p>
                </div>
              )}

              {/* Pattern picker */}
              <div className="grid grid-cols-2 gap-3">
                {DEBUG_PATTERNS.filter((p) => p.id !== "off").map((p) => {
                  const active = (state.debug ?? "off") === p.id;
                  return (
                    <SpotlightCard
                      key={p.id}
                      active={active}
                      spotlightColor="rgba(245, 158, 11, 0.14)"
                      className="p-4 flex flex-col gap-2 h-full"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <h3 className="font-bold text-sm text-white">
                          {p.name}
                        </h3>
                        <button
                          onClick={() => handleSetDebugPattern(p.id)}
                          className={`px-2.5 py-1 rounded-lg border text-[11px] font-semibold transition-all cursor-pointer shrink-0 ${
                            active
                              ? "border-amber-400 bg-amber-500/20 text-amber-300"
                              : "border-white/[0.1] bg-white/[0.04] text-slate-300 hover:bg-white/[0.08]"
                          }`}
                        >
                          {active ? "Running" : "Run"}
                        </button>
                      </div>
                      <p className="text-[11px] font-medium text-slate-300">
                        {p.what}
                      </p>
                      <p className="text-[11px] text-slate-500 leading-relaxed">
                        {p.how}
                      </p>
                    </SpotlightCard>
                  );
                })}
              </div>

              {/* Recommended procedure */}
              <div className="glass-panel rounded-2xl p-5 flex flex-col gap-3">
                <span className="text-xs font-bold uppercase tracking-wider text-slate-300">
                  Suggested Order
                </span>
                <ol className="flex flex-col gap-2 text-[11px] text-slate-400 leading-relaxed list-decimal list-inside">
                  <li>
                    <span className="text-slate-300">All White</span> — count
                    how many LEDs respond at all. This is your upper bound.
                  </li>
                  <li>
                    <span className="text-slate-300">Ruler (every 10th)</span> —
                    use this to count a long strip without losing your place.
                  </li>
                  <li>
                    <span className="text-slate-300">Count Marker</span> — read
                    the exact total from the colour groups.
                  </li>
                  <li>
                    <span className="text-slate-300">Single LED</span> — step
                    through to detect parallel-wired LEDs.
                  </li>
                  <li>
                    <span className="text-slate-300">Snake</span> — walk the
                    pulse to map each index to a physical LED.
                  </li>
                  <li>
                    <span className="text-slate-300">Halves</span> — confirm the
                    addressing direction and any mirroring.
                  </li>
                  <li>
                    Enter the total in{" "}
                    <span className="text-slate-300">
                      Addressable Header Fill Length
                    </span>{" "}
                    on the Lighting tab, then press{" "}
                    <span className="text-slate-300">Stop Test</span>.
                  </li>
                </ol>
                <p className="text-[11px] text-slate-500 pt-2 border-t border-white/[0.06] leading-relaxed">
                  Tip: if the pattern only covers part of the strip, your
                  configured LED count is lower than the physical count. If the
                  strip repeats the pattern early, it is higher.
                </p>
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
