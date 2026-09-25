import React, { useState, useRef, useCallback } from "react";
import { Sliders, RotateCcw } from "lucide-react";

interface RotaryKnobProps {
  value: number; // e.g. 0.2 to 3.0
  min?: number;
  max?: number;
  step?: number;
  defaultValue?: number;
  label?: string;
  unit?: string;
  accentColor?: string;
  onChange: (val: number) => void;
}

export const RotaryKnob: React.FC<RotaryKnobProps> = ({
  value,
  min = 0.2,
  max = 3.0,
  step = 0.05,
  defaultValue = 1.0,
  label = "Sensitivity",
  unit = "x",
  accentColor = "#00d2ff",
  onChange,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef<number>(0);
  const startValRef = useRef<number>(value);
  const knobRef = useRef<HTMLDivElement>(null);

  // Map value to angle (-135 deg to +135 deg = 270 deg sweep)
  const norm = Math.min(1, Math.max(0, (value - min) / (max - min)));
  const startAngle = -135;
  const endAngle = 135;
  const currentAngle = startAngle + norm * (endAngle - startAngle);

  const clampValue = useCallback(
    (v: number) => {
      const rounded = Math.round(v / step) * step;
      return Math.min(max, Math.max(min, Number(rounded.toFixed(2))));
    },
    [min, max, step],
  );

  const handlePointerDown = (e: React.PointerEvent) => {
    setIsDragging(true);
    startYRef.current = e.clientY;
    startValRef.current = value;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging) return;
    const deltaY = startYRef.current - e.clientY; // drag up = increase, drag down = decrease
    const range = max - min;
    const change = (deltaY / 150) * range; // 150px drag covers full range
    const newVal = clampValue(startValRef.current + change);
    if (newVal !== value) {
      onChange(newVal);
    }
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    setIsDragging(false);
    try {
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignore
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY < 0 ? step : -step;
    const newVal = clampValue(value + delta);
    onChange(newVal);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowUp" || e.key === "ArrowRight") {
      e.preventDefault();
      onChange(clampValue(value + step));
    } else if (e.key === "ArrowDown" || e.key === "ArrowLeft") {
      e.preventDefault();
      onChange(clampValue(value - step));
    }
  };

  // SVG Arc calculation for radius 32 in a 76x76 viewBox
  const r = 30;
  const cx = 38;
  const cy = 38;
  const arcSweep = 270;
  const strokeWidth = 3.5;
  const circumference = 2 * Math.PI * r;
  const arcLength = (arcSweep / 360) * circumference;
  const progressOffset = arcLength * (1 - norm);

  // Rotation transform for the arc starting at 135 deg
  // (which is -135 relative to vertical top 0)
  const arcRotation = 135;

  return (
    <div className="flex flex-col items-center select-none">
      {/* Knob Container */}
      <div className="flex items-center gap-3">
        <div
          ref={knobRef}
          tabIndex={0}
          role="slider"
          aria-label={label}
          aria-valuemin={min}
          aria-valuemax={max}
          aria-valuenow={value}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onWheel={handleWheel}
          onKeyDown={handleKeyDown}
          onDoubleClick={() => onChange(defaultValue)}
          title="Drag up/down or scroll wheel to adjust sensitivity. Double click to reset to 1.0x."
          className={`relative h-[76px] w-[76px] rounded-full flex items-center justify-center cursor-ns-resize outline-none transition-all duration-200 ${
            isDragging
              ? "scale-[1.03] ring-2 ring-white/20"
              : "hover:scale-[1.02]"
          }`}
        >
          {/* SVG Circular Progress Track and Active Arc */}
          <svg
            className="absolute inset-0 h-full w-full pointer-events-none"
            viewBox="0 0 76 76"
          >
            {/* Background Arc Track */}
            <circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke="rgba(255, 255, 255, 0.08)"
              strokeWidth={strokeWidth}
              strokeDasharray={`${arcLength} ${circumference}`}
              strokeDashoffset={0}
              strokeLinecap="round"
              transform={`rotate(${arcRotation} ${cx} ${cy})`}
            />

            {/* Glowing Active Arc */}
            <circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke={accentColor}
              strokeWidth={strokeWidth}
              strokeDasharray={`${arcLength} ${circumference}`}
              strokeDashoffset={progressOffset}
              strokeLinecap="round"
              transform={`rotate(${arcRotation} ${cx} ${cy})`}
              style={{
                filter: isDragging
                  ? `drop-shadow(0 0 6px ${accentColor})`
                  : `drop-shadow(0 0 3px ${accentColor}88)`,
                transition: isDragging
                  ? "none"
                  : "stroke-dashoffset 120ms ease-out",
              }}
            />
          </svg>

          {/* Rotary Knob Face */}
          <div
            className="h-[52px] w-[52px] rounded-full bg-gradient-to-b from-slate-800 via-slate-900 to-slate-950 border border-white/[0.12] shadow-[inset_0_1px_1px_rgba(255,255,255,0.15),0_6px_16px_rgba(0,0,0,0.6)] flex items-center justify-center relative overflow-hidden transition-transform duration-75"
            style={{ transform: `rotate(${currentAngle}deg)` }}
          >
            {/* Radial metallic texture effect */}
            <div className="absolute inset-0 rounded-full opacity-30 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.15)_0%,transparent_70%)]" />

            {/* Indicator notch / tick line */}
            <div
              className="absolute top-1.5 h-2.5 w-1 rounded-full shadow-[0_0_6px_#fff]"
              style={{ backgroundColor: accentColor }}
            />
          </div>

          {/* Center Digital Value Readout */}
          <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
            <span className="text-[11px] font-bold font-mono tracking-tight text-white drop-shadow-[0_1px_3px_rgba(0,0,0,0.9)]">
              {value.toFixed(1)}
              <span className="text-[9px] font-normal text-slate-400">
                {unit}
              </span>
            </span>
          </div>
        </div>

        {/* Right side: Labels & Quick Presets */}
        <div className="flex flex-col gap-1.5 min-w-[100px]">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-1">
              <Sliders className="h-2.5 w-2.5 text-cyan-400" />
              {label}
            </span>
            <button
              onClick={() => onChange(defaultValue)}
              title="Reset to 1.0x"
              className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors cursor-pointer p-0.5"
            >
              <RotateCcw className="h-2.5 w-2.5" />
            </button>
          </div>

          {/* Preset Buttons */}
          <div className="grid grid-cols-4 gap-1">
            {[0.6, 1.0, 1.6, 2.4].map((preset) => {
              const isActive = Math.abs(value - preset) < 0.04;
              return (
                <button
                  key={preset}
                  onClick={() => onChange(preset)}
                  className={`py-0.5 rounded text-[9px] font-mono font-medium transition-all cursor-pointer ${
                    isActive
                      ? "bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 shadow-[0_0_8px_rgba(6,182,212,0.2)]"
                      : "bg-white/[0.04] text-slate-400 hover:text-slate-200 hover:bg-white/[0.08] border border-white/[0.06]"
                  }`}
                >
                  {preset}x
                </button>
              );
            })}
          </div>

          <span className="text-[9px] text-slate-500 leading-tight">
            {value < 0.85
              ? "Subtle - quieter passages stay calm"
              : value > 1.5
                ? "Boost - reacts to faint detail"
                : "Default - tuned for typical music"}
          </span>
        </div>
      </div>
    </div>
  );
};
