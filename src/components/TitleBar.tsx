import React, { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Minus,
  Square,
  X,
  Copy,
  Zap,
  Power,
  RefreshCw,
  Clock,
} from "lucide-react";

interface TitleBarProps {
  connected: boolean;
  currentColorHex: string;
  isOff: boolean;
  onTogglePower: () => void;
  onStartOpenRgb: () => void;
  onStopArmoury: () => void;
  startingOpenRgb: boolean;
  sleepTimerMinutes: number | null;
  onCancelSleepTimer: () => void;
}

export const TitleBar: React.FC<TitleBarProps> = ({
  connected,
  currentColorHex,
  isOff,
  onTogglePower,
  onStartOpenRgb,
  onStopArmoury,
  startingOpenRgb,
  sleepTimerMinutes,
  onCancelSleepTimer,
}) => {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const checkMaximized = async () => {
      try {
        const win = getCurrentWindow();
        const max = await win.isMaximized();
        setIsMaximized(max);
      } catch {
        // Fallback for non-tauri dev environment
      }
    };
    checkMaximized();
  }, []);

  const handleMinimize = async () => {
    try {
      const win = getCurrentWindow();
      await win.minimize();
    } catch (e) {
      console.warn(e);
    }
  };

  const handleMaximize = async () => {
    try {
      const win = getCurrentWindow();
      await win.toggleMaximize();
      setIsMaximized(await win.isMaximized());
    } catch (e) {
      console.warn(e);
    }
  };

  const handleClose = async () => {
    try {
      const win = getCurrentWindow();
      await win.close();
    } catch (e) {
      console.warn(e);
    }
  };

  const handleMouseDown = async (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (
      target.closest("button") ||
      target.closest("input") ||
      target.closest("a") ||
      target.closest("[role='button']")
    ) {
      return;
    }
    try {
      const win = getCurrentWindow();
      await win.startDragging();
    } catch (err) {
      console.warn("startDragging error:", err);
    }
  };

  return (
    <header
      data-tauri-drag-region
      onMouseDown={handleMouseDown}
      onDoubleClick={handleMaximize}
      className="h-12 w-full select-none flex items-center justify-between px-3 border-b border-white/[0.08] bg-[#07090e] z-50 shrink-0 cursor-default"
    >
      {/* Left: Branding & Status Badge */}
      <div className="flex items-center gap-2.5 pointer-events-none">
        {/* Dynamic Logo Icon */}
        <div
          className="h-7 w-7 rounded-lg flex items-center justify-center transition-all duration-700 relative overflow-hidden p-0.5"
          style={{
            backgroundColor: isOff ? "#1e293b" : `${currentColorHex}22`,
            boxShadow: isOff ? "none" : `0 0 16px ${currentColorHex}88`,
            border: `1px solid ${isOff ? "#334155" : currentColorHex}`,
          }}
        >
          <img
            src="/app-icon.png"
            alt="betterRGB"
            className="h-full w-full object-contain rounded"
          />
        </div>

        <div className="flex items-center gap-2">
          <span className="font-semibold text-xs tracking-tight bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent">
            betterRGB
          </span>
          <span className="text-[10px] font-medium tracking-wide uppercase px-1.5 py-0.5 rounded bg-white/[0.06] border border-white/[0.08] text-slate-400">
            PRIME B760M-A
          </span>
        </div>

        {/* Server status pill */}
        <div
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium border ml-1 ${
            connected
              ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-400"
              : "bg-amber-500/10 border-amber-500/30 text-amber-300"
          }`}
        >
          <div
            className={`h-1.5 w-1.5 rounded-full ${
              connected
                ? "bg-emerald-400 shadow-[0_0_6px_#34d399]"
                : "bg-amber-400"
            }`}
          />
          <span>{connected ? "SDK Connected" : "Connecting..."}</span>
        </div>

        {/* Sleep Timer Indicator */}
        {sleepTimerMinutes !== null && (
          <button
            onClick={onCancelSleepTimer}
            title="Click to cancel sleep timer"
            className="pointer-events-auto flex items-center gap-1 px-2 py-0.5 rounded-full bg-cyan-500/15 border border-cyan-500/30 text-cyan-300 text-[10px] hover:bg-cyan-500/25 transition-all cursor-pointer"
          >
            <Clock className="h-3 w-3" />
            <span>Auto-Off: {sleepTimerMinutes}m</span>
          </button>
        )}
      </div>

      {/* Center: Draggable Spacer */}
      <div data-tauri-drag-region className="flex-1 h-full" />

      {/* Right: Quick Action Pills & Window Controls */}
      <div className="flex items-center gap-1">
        {/* If OpenRGB not connected, show quick launch */}
        {!connected && (
          <button
            onClick={onStartOpenRgb}
            disabled={startingOpenRgb}
            className="flex items-center gap-1 px-2.5 py-1 rounded-md bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/30 text-cyan-300 text-xs font-medium transition-all mr-1 cursor-pointer disabled:opacity-50"
          >
            <Zap className="h-3 w-3" />
            <span>{startingOpenRgb ? "Starting..." : "Start OpenRGB"}</span>
          </button>
        )}

        {/* Armoury Crate Pause */}
        <button
          onClick={onStopArmoury}
          title="Pause Asus LightingService if it conflicts with motherboard headers"
          className="flex items-center gap-1 px-2 py-1 rounded-md bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] text-slate-400 hover:text-slate-200 text-xs transition-all cursor-pointer mr-1"
        >
          <RefreshCw className="h-3 w-3" />
          <span className="text-[11px]">Free Headers</span>
        </button>

        {/* Quick Blackout / Power Toggle */}
        <button
          onClick={onTogglePower}
          title={isOff ? "Turn Lighting ON" : "Instant Blackout (Turn OFF)"}
          className={`flex items-center gap-1 px-2.5 py-1 rounded-md border text-xs font-semibold transition-all cursor-pointer mr-2 ${
            isOff
              ? "bg-red-500/15 border-red-500/30 text-red-400 hover:bg-red-500/25"
              : "bg-white/[0.06] border-white/[0.1] text-slate-300 hover:text-white hover:bg-white/[0.1]"
          }`}
        >
          <Power className="h-3 w-3" />
          <span className="text-[11px]">{isOff ? "LEDs OFF" : "Blackout"}</span>
        </button>

        {/* Native-style Window Control Buttons */}
        <div className="flex items-center">
          <button
            onClick={handleMinimize}
            className="h-8 w-8 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
            title="Minimize"
          >
            <Minus className="h-3.5 w-3.5" />
          </button>

          <button
            onClick={handleMaximize}
            className="h-8 w-8 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-white/[0.08] transition-colors cursor-pointer"
            title={isMaximized ? "Restore" : "Maximize"}
          >
            {isMaximized ? (
              <Copy className="h-3 w-3" />
            ) : (
              <Square className="h-3 w-3" />
            )}
          </button>

          <button
            onClick={handleClose}
            className="h-8 w-8 flex items-center justify-center rounded text-slate-400 hover:text-white hover:bg-red-600 transition-colors cursor-pointer"
            title="Close"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </header>
  );
};
