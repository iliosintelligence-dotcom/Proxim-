"use client";

import { useRef, useState } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";

/** FAQ list. Answers open with a height tween; the plus turns into a minus. */
export function Accordion({ items }: { items: { q: string; a: string }[] }) {
  const [open, setOpen] = useState<number | null>(0);
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const panels = gsap.utils.toArray<HTMLElement>("[data-acc-panel]");
      panels.forEach((p, i) => {
        gsap.to(p, {
          height: i === open ? "auto" : 0,
          opacity: i === open ? 1 : 0,
          duration: prefersReducedMotion() ? 0 : 0.55,
          ease: "expo.out",
        });
      });
    },
    { scope: root, dependencies: [open] },
  );

  return (
    <div ref={root} className="divide-y border-y" style={{ borderColor: "var(--line)" }}>
      {items.map((it, i) => {
        const on = open === i;
        return (
          <div key={it.q} data-reveal style={{ borderColor: "var(--line)" }}>
            <h3>
              <button
                type="button"
                aria-expanded={on}
                aria-controls={`acc-${i}`}
                onClick={() => setOpen(on ? null : i)}
                className="flex w-full items-center justify-between gap-6 py-5 text-left text-[1.05rem] font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue"
              >
                {it.q}
                <span className="relative grid size-7 shrink-0 place-items-center rounded-full border border-[var(--line-strong)]">
                  <span className="absolute h-[2px] w-2.5 bg-ink" />
                  <span
                    className={`absolute h-2.5 w-[2px] bg-ink transition-transform duration-300 ease-[var(--ease-out-quint)] ${on ? "scale-y-0" : ""}`}
                  />
                </span>
              </button>
            </h3>
            <div id={`acc-${i}`} data-acc-panel className={`overflow-hidden ${on ? "" : "h-0 opacity-0"}`} role="region">
              <p className="max-w-[46rem] pb-6 text-[0.98rem] leading-relaxed text-muted-foreground">{it.a}</p>
            </div>
          </div>
        );
      })}
    </div>
  );
}
