import { PipMark } from "@/components/mascot/PipShape";

/**
 * The logo is Pip: the mascot is the brand. The mark is the smooth pin
 * with its face; the wordmark is set in Momo Trust Display, the same
 * heavy, rounded voice as the headlines.
 */
export function LogoMark({ className = "", night = false }: { className?: string; night?: boolean }) {
  return <PipMark className={className} cheeks={false} color={night ? "#ffffff" : undefined} />;
}

export function Logo({ night = false }: { night?: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <LogoMark className="h-6 w-auto" night={night} />
      <span className={`font-display text-[1.4rem] leading-none tracking-[-0.03em] ${night ? "text-white" : "text-ink"}`}>proxim</span>
    </span>
  );
}
