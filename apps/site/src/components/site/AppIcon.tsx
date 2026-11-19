import type { LucideIcon } from "lucide-react";

const tones = {
  blue: "from-[#5b95ff] to-[#0047e0]",
  ink: "from-[#4a4a4a] to-[#0d0d0d]",
  green: "from-[#46e08f] to-[#0f9d58]",
  orange: "from-[#ffb357] to-[#ff5f1f]",
  pink: "from-[#ff86c1] to-[#e2337f]",
  purple: "from-[#b49cff] to-[#6a3df0]",
  yellow: "from-[#ffe680] to-[#ffbf1a]",
  teal: "from-[#5ee3e0] to-[#0ea5a8]",
} as const;

export type IconTone = keyof typeof tones;

const sizes = {
  sm: "size-8",
  md: "size-11",
  lg: "size-14",
};

/**
 * An icon drawn the way iPhone app icons are: a continuous-corner
 * squircle with a soft top-to-bottom gradient, a faint gloss, and a clean
 * white symbol. Tones are picked by meaning, like apps on a home screen.
 */
export function AppIcon({
  icon: Icon,
  tone = "blue",
  size = "md",
  className = "",
}: {
  icon: LucideIcon;
  tone?: IconTone;
  size?: keyof typeof sizes;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={`relative grid shrink-0 place-items-center overflow-hidden rounded-[27%] bg-gradient-to-b shadow-[inset_0_1px_0_rgb(255_255_255/0.35),inset_0_-1px_0_rgb(0_0_0/0.12),0_8px_18px_-8px_rgb(0_0_0/0.4)] ${tones[tone]} ${sizes[size]} ${className}`}
    >
      <span className="absolute inset-x-0 top-0 h-1/2 bg-gradient-to-b from-white/22 to-transparent" />
      <Icon className={`relative ${tone === "yellow" ? "text-ink" : "text-white"} size-[52%]`} strokeWidth={2.1} />
    </span>
  );
}
