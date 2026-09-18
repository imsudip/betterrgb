import React from "react";

interface TelemetrySparklineProps {
  data: number[];
  color?: string;
  min?: number;
  max?: number;
  height?: number;
}

export const TelemetrySparkline: React.FC<TelemetrySparklineProps> = ({
  data,
  color = "#00d2ff",
  min = 0,
  max = 100,
  height = 48,
}) => {
  if (!data || data.length < 2) {
    return (
      <div
        className="w-full flex items-center justify-center text-[10px] text-slate-500"
        style={{ height }}
      >
        Awaiting telemetry...
      </div>
    );
  }

  const width = 180;
  const padding = 4;
  const effectiveHeight = height - padding * 2;

  // Normalize data points
  const points = data.map((val, idx) => {
    const x = padding + (idx / (data.length - 1)) * (width - padding * 2);
    const normalized = Math.max(0, Math.min(1, (val - min) / (max - min || 1)));
    const y = padding + (1 - normalized) * effectiveHeight;
    return { x, y, val };
  });

  const pathD = points.reduce((acc, pt, i) => {
    return i === 0 ? `M ${pt.x},${pt.y}` : `${acc} L ${pt.x},${pt.y}`;
  }, "");

  const areaD = `${pathD} L ${points[points.length - 1].x},${height} L ${points[0].x},${height} Z`;

  const lastPoint = points[points.length - 1];

  return (
    <div className="relative w-full overflow-hidden" style={{ height }}>
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full h-full overflow-visible"
        preserveAspectRatio="none"
      >
        <defs>
          <linearGradient
            id={`sparkline-grad-${color.replace(/[^a-zA-Z0-9]/g, "")}`}
            x1="0"
            y1="0"
            x2="0"
            y2="1"
          >
            <stop offset="0%" stopColor={color} stopOpacity="0.3" />
            <stop offset="100%" stopColor={color} stopOpacity="0.0" />
          </linearGradient>
        </defs>

        {/* Gradient fill beneath line */}
        <path
          d={areaD}
          fill={`url(#sparkline-grad-${color.replace(/[^a-zA-Z0-9]/g, "")})`}
        />

        {/* Main sparkline */}
        <path
          d={pathD}
          fill="none"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Current reading pulse dot */}
        {lastPoint && (
          <circle
            cx={lastPoint.x}
            cy={lastPoint.y}
            r="3"
            fill={color}
            className="animate-pulse"
          />
        )}
      </svg>
    </div>
  );
};
