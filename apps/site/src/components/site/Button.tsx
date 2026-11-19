import Link from "next/link";
import { ArrowRight } from "lucide-react";
import type { ReactNode } from "react";

type Variant = "primary" | "secondary" | "night";

const styles: Record<Variant, string> = {
  primary: "bg-ink text-white hover:bg-[#262626]",
  secondary: "bg-stone-2 text-ink hover:bg-[#d6d6d6]",
  night: "bg-white text-ink hover:bg-[#ececec]",
};

const sizes = {
  md: { base: "h-11 text-[0.95rem]", pad: "px-5", padArrow: "pl-5 pr-1.5", tile: "size-8" },
  sm: { base: "h-9 text-[0.875rem]", pad: "px-4", padArrow: "pl-4 pr-1", tile: "size-7" },
};

/**
 * Ledger-style button. Primary actions carry a small yellow square with
 * an arrow (the one place yellow appears before the finale). On hover the
 * label rolls up and an identical one rolls in from below, and the arrow
 * steps forward: a quiet confirmation that it will take you somewhere.
 */
export function Button({
  href,
  children,
  variant = "primary",
  size = "md",
  arrow = false,
  className = "",
}: {
  href: string;
  children: ReactNode;
  variant?: Variant;
  size?: keyof typeof sizes;
  arrow?: boolean;
  className?: string;
}) {
  const s = sizes[size];
  return (
    <Link
      href={href}
      className={`group inline-flex items-center justify-center gap-3 rounded-lg font-medium whitespace-nowrap transition-[background-color,transform] duration-300 ease-[var(--ease-out-quint)] active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${styles[variant]} ${s.base} ${arrow ? s.padArrow : s.pad} ${className}`}
    >
      <RollingLabel>{children}</RollingLabel>
      {arrow && (
        <span className={`grid shrink-0 place-items-center rounded-md bg-yellow text-ink transition-transform duration-300 ease-[var(--ease-out-quint)] group-hover:translate-x-0.5 ${s.tile}`}>
          <TriangleArrow />
        </span>
      )}
    </Link>
  );
}

/** The label twice, stacked; hovering slides the stack up by one line. */
export function RollingLabel({ children }: { children: ReactNode }) {
  return (
    <span className="relative block overflow-hidden leading-[1.25]">
      <span className="block transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-full">
        {children}
      </span>
      <span
        aria-hidden="true"
        className="absolute inset-x-0 top-full block transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-full"
      >
        {children}
      </span>
    </span>
  );
}

export function TriangleArrow({ className = "" }: { className?: string }) {
  return (
    <svg viewBox="0 0 10 10" className={`size-[0.6em] ${className}`} aria-hidden="true">
      <path d="M2.5 1.2v7.6L8.6 5z" fill="currentColor" />
    </svg>
  );
}

/** Small smooth arrow for inline links; steps forward on hover. */
export function PixelArrow({ className = "" }: { className?: string }) {
  return (
    <ArrowRight
      aria-hidden="true"
      strokeWidth={2.2}
      className={`size-[1em] transition-transform duration-200 ease-[var(--ease-out-quint)] group-hover:translate-x-[3px] ${className}`}
    />
  );
}
