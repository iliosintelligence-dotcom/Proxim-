"use client";

import { useRef, type ReactNode } from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";

/**
 * An endless row that responds to the reader. It drifts slowly on its
 * own, speeds up with the speed of your scroll, and reverses when you
 * scroll back up, then settles again. Hovering pauses it so a line can be
 * read.
 */
export function Marquee({
  children,
  seconds = 40,
  reverse = false,
  className = "",
  gap = "gap-3",
  vertical = false,
}: {
  /** Run up/down instead of sideways (for columns). */
  vertical?: boolean;
  children: ReactNode;
  /** Time for one full loop at rest. */
  seconds?: number;
  reverse?: boolean;
  className?: string;
  gap?: string;
}) {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const track = root.current!.querySelector<HTMLElement>("[data-track]")!;
      const axis = vertical ? "yPercent" : "xPercent";
      const tween = gsap.fromTo(
        track,
        { [axis]: reverse ? -50 : 0 },
        { [axis]: reverse ? 0 : -50, duration: seconds, ease: "none", repeat: -1 },
      );
      let paused = false;
      ScrollTrigger.create({
        trigger: root.current,
        start: "top bottom",
        end: "bottom top",
        // Calculated after any pinned section above it, so its positions
        // include the pin's extra scroll and it never pauses by mistake.
        refreshPriority: -1,
        onUpdate: (self) => {
          if (paused) return;
          // Scroll down: faster forward. Scroll up: runs backwards. Then settles.
          const dir = self.direction;
          const boost = 1 + Math.min(6, Math.abs(self.getVelocity()) / 260);
          gsap.to(tween, {
            timeScale: dir * boost,
            duration: 0.2,
            overwrite: true,
            onComplete: () => {
              gsap.to(tween, { timeScale: dir, duration: 1.2, ease: "power2.out" });
            },
          });
        },
        onToggle: (self) => (self.isActive ? tween.resume() : tween.pause()),
      });
      const el = root.current!;
      const enter = () => {
        paused = true;
        gsap.to(tween, { timeScale: 0, duration: 0.5, overwrite: true });
      };
      const leave = () => {
        paused = false;
        gsap.to(tween, { timeScale: 1, duration: 0.6, overwrite: true });
      };
      el.addEventListener("pointerenter", enter);
      el.addEventListener("pointerleave", leave);
      return () => {
        el.removeEventListener("pointerenter", enter);
        el.removeEventListener("pointerleave", leave);
      };
    },
    { scope: root, dependencies: [vertical], lazy: true },
  );

  return (
    <div ref={root} className={`overflow-hidden ${className}`}>
      <div data-track className={`flex ${vertical ? "flex-col" : "w-max"} ${gap}`}>
        <div className={`flex shrink-0 ${vertical ? "flex-col" : "items-center"} ${gap}`}>{children}</div>
        <div className={`flex shrink-0 ${vertical ? "flex-col" : "items-center"} ${gap}`} aria-hidden="true">
          {children}
        </div>
      </div>
    </div>
  );
}
