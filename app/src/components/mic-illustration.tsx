import type { RgbColor } from "@/lib/bindings";
import { m } from "@/paraglide/messages";

export const RGB_HEX: Record<RgbColor, string> = {
  white: "#f4f5f7",
  red: "#ef4444",
  orange: "#f97316",
  yellow: "#eab308",
  green: "#22c55e",
  cyan: "#06b6d4",
  blue: "#3b82f6",
  purple: "#a855f7",
  rainbow: "#c084fc",
};

/**
 * The PD200X on its boom arm, the ring light in the chosen colour. Dark when
 * the light is off, or muted with "light off while muted".
 */
export function MicIllustration({ color, brightness, lit }: { color: RgbColor; brightness: number; lit: boolean }) {
  const b = lit ? brightness / 100 : 0;
  const ringColor = RGB_HEX[color];
  const ringFill = color === "rainbow" ? "url(#mic-rainbow)" : ringColor;
  const ringOpacity = lit ? 0.3 + b * 0.7 : 0.12;
  const glowOpacity = lit ? 0.15 + b * 0.75 : 0;
  const floorOpacity = b * 0.22;
  return (
    <svg
      viewBox="0 0 560 520"
      overflow="visible"
      className="h-auto w-full max-w-[560px]"
      role="img"
      aria-label={m.light_preview_label()}
    >
      <defs>
        <linearGradient id="mic-body" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#454952" />
          <stop offset="0.28" stopColor="#2a2d33" />
          <stop offset="1" stopColor="#0f1012" />
        </linearGradient>
        <linearGradient id="mic-cap" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#363a40" />
          <stop offset="1" stopColor="#0e0f11" />
        </linearGradient>
        <linearGradient id="mic-band" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#1a1c20" />
          <stop offset="0.5" stopColor="#3a3e45" />
          <stop offset="1" stopColor="#16181b" />
        </linearGradient>
        <linearGradient id="mic-boom" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#3a3e45" />
          <stop offset="1" stopColor="#15171a" />
        </linearGradient>
        <radialGradient id="mic-foamShade" cx="0.35" cy="0.3" r="0.85">
          <stop offset="0" stopColor="#303236" />
          <stop offset="1" stopColor="#0c0d0f" />
        </radialGradient>
        <radialGradient id="mic-knob" cx="0.4" cy="0.35" r="0.7">
          <stop offset="0" stopColor="#4a4e56" />
          <stop offset="1" stopColor="#17191c" />
        </radialGradient>
        <pattern id="mic-foamDots" width="5" height="5" patternUnits="userSpaceOnUse">
          <circle cx="2.5" cy="2.5" r="0.9" fill="#000" opacity="0.5" />
        </pattern>
        <linearGradient id="mic-ringShade" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#fff" stopOpacity="0.45" />
          <stop offset="0.4" stopColor="#fff" stopOpacity="0" />
          <stop offset="1" stopColor="#000" stopOpacity="0.35" />
        </linearGradient>
        <linearGradient id="mic-rainbow" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ef4444" />
          <stop offset="0.2" stopColor="#f59e0b" />
          <stop offset="0.4" stopColor="#22c55e" />
          <stop offset="0.6" stopColor="#06b6d4" />
          <stop offset="0.8" stopColor="#3b82f6" />
          <stop offset="1" stopColor="#a855f7" />
        </linearGradient>
        <filter id="mic-glow" x="-300%" y="-60%" width="700%" height="220%">
          <feGaussianBlur stdDeviation="16" />
        </filter>
        <filter id="mic-soft" x="-100%" y="-100%" width="300%" height="300%">
          <feGaussianBlur stdDeviation="40" />
        </filter>
      </defs>
      <ellipse cx="250" cy="330" rx="150" ry="120" fill={ringColor} opacity={floorOpacity} filter="url(#mic-soft)" />
      <g transform="rotate(-38 330 112)">
        <rect x="330" y="99" width="900" height="26" rx="13" fill="url(#mic-boom)" />
      </g>
      <rect x="306" y="110" width="20" height="92" rx="6" fill="#26292e" />
      <circle cx="330" cy="112" r="20" fill="#2c2f35" stroke="#45494f" strokeWidth="2" />
      <circle cx="330" cy="112" r="7" fill="#16181b" />
      <g transform="rotate(-35 300 300)">
        <rect
          x="196"
          y="222"
          width="30"
          height="156"
          rx="12"
          fill={ringFill}
          opacity={glowOpacity}
          filter="url(#mic-glow)"
        />
        <rect x="436" y="246" width="22" height="108" rx="9" fill="url(#mic-cap)" />
        <rect x="212" y="232" width="228" height="136" rx="26" fill="url(#mic-body)" />
        <rect x="230" y="239" width="194" height="5" rx="2.5" fill="#fff" opacity="0.09" />
        <circle cx="330" cy="272" r="25" fill="none" stroke="#4ade80" strokeWidth="3" />
        <circle cx="330" cy="272" r="25" fill="none" stroke="#4ade80" strokeWidth="8" opacity="0.18" />
        <circle cx="330" cy="272" r="19" fill="url(#mic-knob)" stroke="#0e0f11" strokeWidth="2" />
        <rect x="262" y="263" width="24" height="18" rx="6" fill="#7f1d1d" stroke="#0e0f11" strokeWidth="2" />
        <rect x="268" y="270" width="12" height="4" rx="2" fill="#fca5a5" opacity="0.7" />
        <rect x="366" y="220" width="26" height="160" rx="11" fill="url(#mic-band)" />
        <rect x="40" y="222" width="168" height="156" rx="70" fill="url(#mic-foamShade)" />
        <rect x="40" y="222" width="168" height="156" rx="70" fill="url(#mic-foamDots)" />
        <rect x="200" y="228" width="18" height="144" rx="7" fill={ringFill} opacity={ringOpacity} />
        <rect x="200" y="228" width="18" height="144" rx="7" fill="url(#mic-ringShade)" />
      </g>
    </svg>
  );
}
