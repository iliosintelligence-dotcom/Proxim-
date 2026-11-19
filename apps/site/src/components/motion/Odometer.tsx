"use client";

import { useRef } from "react";
import type React from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";

/**
 * Rolling number. Each digit is a strip of 0-9 that slides to its value,
 * the same way an impression counter ticks over, so changing a slider
 * feels like turning a dial rather than swapping text.
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
  const root = useRef<HTMLSpanElement>(null);
  const firstRun = useRef(true);
  const chars = value.split("");
  // Key digits from the right so "9,900" -> "10,100" keeps columns stable.
  const keyed = chars.map((c, i) => ({ c, key: chars.length - i }));

  useGSAP(
    () => {
      const strips = root.current!.querySelectorAll<HTMLElement>("[data-strip]");
      const reduce = prefersReducedMotion();
      const instant = firstRun.current;
      firstRun.current = false;
      strips.forEach((strip, i) => {
        const d = Number(strip.dataset.digit);
        if (instant || !strip.dataset.ready) {
          // Server rendered this strip with a CSS transform; hand it to GSAP
          // without a visible jump.
          gsap.set(strip, { y: 0, yPercent: -d * 10 });
          strip.dataset.ready = "1";
          return;
        }
        gsap.to(strip, {
          yPercent: -d * 10,
          duration: reduce ? 0 : 1.1,
          ease: "expo.out",
          delay: reduce ? 0 : (strips.length - i) * 0.025,
          overwrite: true,
        });
      });
    },
    { scope: root, dependencies: [value] },
  );

  return (
    <span
      ref={root}
      className={`inline-flex tabular-nums ${className}`}
      aria-label={ariaLabel ?? value}
      role="text"
    >
      {keyed.map(({ c, key }) =>
        /\d/.test(c) ? (
          <span
            key={key}
            aria-hidden="true"
            className="relative inline-block h-[1em] overflow-hidden leading-none"
            style={{ width: "0.62em" }}
          >
            <span
              data-strip
              data-digit={c}
              // The class positions the strip before hydration. After that GSAP
              // owns the inline transform, which wins over the class.
              className="absolute inset-x-0 top-0 flex flex-col [transform:translateY(calc(var(--d)*-10%))]"
              style={{ "--d": c } as React.CSSProperties}
            >
              {Array.from({ length: 10 }, (_, n) => (
                <span key={n} className="block h-[1em] text-center leading-none">
                  {n}
                </span>
              ))}
            </span>
          </span>
        ) : (
          <span key={`s${key}`} aria-hidden="true" className="inline-block leading-none">
            {c}
          </span>
        ),
      )}
    </span>
  );
}
