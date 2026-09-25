import React, { memo, useRef } from "react";

interface SpotlightCardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  className?: string;
  spotlightColor?: string;
  active?: boolean;
}

/** Cursor-following spotlight card. Position is written via CSS vars to avoid re-rendering on mousemove. */
export const SpotlightCard: React.FC<SpotlightCardProps> = memo(
  function SpotlightCard({
    children,
    className = "",
    spotlightColor = "rgba(0, 210, 255, 0.12)",
    active = false,
    style,
    onMouseEnter,
    onMouseLeave,
    ...props
  }) {
    const divRef = useRef<HTMLDivElement>(null);

    const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
      const el = divRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      el.style.setProperty("--spot-x", `${e.clientX - rect.left}px`);
      el.style.setProperty("--spot-y", `${e.clientY - rect.top}px`);
    };

    const handleMouseEnter = (e: React.MouseEvent<HTMLDivElement>) => {
      divRef.current?.style.setProperty("--spot-opacity", "1");
      onMouseEnter?.(e);
    };

    const handleMouseLeave = (e: React.MouseEvent<HTMLDivElement>) => {
      divRef.current?.style.setProperty("--spot-opacity", "0");
      onMouseLeave?.(e);
    };

    return (
      <div
        ref={divRef}
        onMouseMove={handleMouseMove}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        className={`relative overflow-hidden rounded-2xl border transition-colors duration-300 ${className} ${
          active
            ? "border-cyan-400/50 bg-slate-900/90 shadow-[0_0_24px_rgba(0,210,255,0.18)]"
            : "border-white/[0.08] bg-slate-900/80 hover:border-white/[0.18] hover:bg-slate-900/95"
        }`}
        style={
          {
            "--spot-x": "0px",
            "--spot-y": "0px",
            "--spot-opacity": "0",
            ...style,
          } as React.CSSProperties
        }
        {...props}
      >
        {/* Specular top-edge shine */}
        <div
          className="absolute inset-x-0 top-0 h-[1px] pointer-events-none"
          style={{
            background:
              "linear-gradient(to right, transparent, rgba(255,255,255,0.2), transparent)",
          }}
        />

        {/* Spotlight glow, driven by CSS vars so mousemove causes no re-render. */}
        <div
          className="pointer-events-none absolute -inset-px rounded-2xl transition-opacity duration-300"
          style={{
            opacity: "var(--spot-opacity)",
            background: `radial-gradient(400px circle at var(--spot-x) var(--spot-y), ${spotlightColor}, transparent 80%)`,
          }}
        />

        <div className="relative z-10">{children}</div>
      </div>
    );
  },
);
