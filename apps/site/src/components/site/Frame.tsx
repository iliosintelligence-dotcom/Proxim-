import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

/**
 * The page column. On wide screens it carries two hairline guide rails,
 * the same quiet structure Audienceful uses, so every section lines up.
 */
export function Frame({
  children,
  className = "",
  night = false,
}: {
  children: ReactNode;
  className?: string;
  night?: boolean;
}) {
  return (
    <div
      className={`relative mx-auto w-full max-w-[1240px] px-[var(--gutter)] lg:border-x ${className}`}
      style={{ borderColor: night ? "var(--line-night)" : "var(--line)" }}
    >
      {children}
    </div>
  );
}

type Tone = "paper" | "stone" | "night";

const toneClass: Record<Tone, string> = {
  paper: "bg-paper text-ink",
  stone: "bg-stone text-ink",
  night: "bg-night text-white",
};

/** A page section with the shared rhythm: top rule, label, generous padding. */
export function Section({
  id,
  index,
  label,
  tone = "paper",
  children,
  className = "",
  frameClassName = "",
  icon,
  pip,
}: {
  /** What Pip says while this section is in view. */
  pip?: string;
  /** Icon shown in the section label. */
  icon?: LucideIcon;
  id?: string;
  index?: string;
  label?: string;
  tone?: Tone;
  children: ReactNode;
  className?: string;
  frameClassName?: string;
}) {
  const night = tone === "night";
  return (
    <section id={id} data-pip={pip} data-screen={night || undefined} className={`${toneClass[tone]} ${className} relative scroll-mt-16 overflow-clip`}>
      <div className="relative border-t" style={{ borderColor: night ? "var(--line-night)" : "var(--line)" }}>
        <Frame night={night} className={`py-[clamp(4.5rem,10vw,8.5rem)] ${frameClassName}`}>
          {label && <SectionLabel index={index} label={label} night={night} icon={icon} />}
          {children}
        </Frame>
      </div>
    </section>
  );
}

export function SectionLabel({
  index: _index,
  label,
  night = false,
  icon: Icon,
}: {
  index?: string;
  label: string;
  night?: boolean;
  icon?: LucideIcon;
}) {
  // Ledger's quiet "/ LABEL" style, with the section's icon in front.
  void _index;
  return (
    <p data-reveal className={`t-label mb-6 inline-flex items-center gap-2 md:mb-8 ${night ? "text-white/80" : "text-ink"}`}>
      {Icon && <Icon className={`size-4 ${night ? "text-white/70" : "text-blue"}`} strokeWidth={2} />}
      <span>/ {label}</span>
    </p>
  );
}
