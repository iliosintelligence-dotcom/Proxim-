import type React from "react";

/**
 * Rolling number. Each digit is a strip of 0-9 that slides to its value,
 * the same way an impression counter ticks over, so changing a slider
 * feels like turning a dial rather than swapping text.
 *
 * The roll is a plain CSS transition on each strip. No script runs when a
 * counter mounts, changes or leaves, which keeps page changes fast on
 * pages with many counters (Pricing has a dozen).
 */
export function Odometer({
  value,
  className = "",
  ariaLabel,
}: {
  value: string;
  className?: string;
  ariaLabel?: string;
}) {
  const chars = value.split("");
  // Key digits from the right so "9,900" -> "10,100" keeps columns stable.
  const keyed = chars.map((c, i) => ({ c, key: chars.length - i }));
  const digits = chars.filter((c) => /\d/.test(c)).length;
  let seen = 0;

  return (
    <span className={`inline-flex tabular-nums ${className}`} aria-label={ariaLabel ?? value} role="text">
      {keyed.map(({ c, key }) => {
        if (!/\d/.test(c)) {
          return (
            <span key={`s${key}`} aria-hidden="true" className="inline-block leading-none">
              {c}
            </span>
          );
        }
        // Right-hand digits settle first, like a real counter.
        const delay = (digits - seen++) * 25;
        return (
          <span key={key} aria-hidden="true" className="relative inline-block h-[1em] overflow-hidden leading-none" style={{ width: "0.62em" }}>
            <span
              className="absolute inset-x-0 top-0 flex flex-col transition-transform duration-[1100ms] ease-[cubic-bezier(0.16,1,0.3,1)] [transform:translateY(calc(var(--d)*-10%))] motion-reduce:transition-none"
              style={{ "--d": c, transitionDelay: `${delay}ms` } as React.CSSProperties}
            >
              {Array.from({ length: 10 }, (_, n) => (
                <span key={n} className="block h-[1em] text-center leading-none">
                  {n}
                </span>
              ))}
            </span>
          </span>
        );
      })}
    </span>
  );
}
