"use client";

import { useRef } from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import type { Format, Niche } from "@/lib/data";
import { Phone } from "./Phone";
import { AppScreen, LockScreen } from "./AppScreens";

/**
 * A phone that reacts to choices. When the niche or format changes, the
 * app underneath swaps and the ad lands again, so every control on the
 * page has a visible result.
 */
export function LivePhone({
  niche,
  format,
  screen = "app",
  host,
  replayKey = 0,
  showAd = true,
  className = "",
}: {
  niche: Niche;
  format: Format;
  screen?: "app" | "lock";
  host?: Niche["id"];
  replayKey?: number;
  showAd?: boolean;
  className?: string;
}) {
  const root = useRef<HTMLDivElement>(null);
  const landed = useRef(false);

  useGSAP(
    () => {
      const ad = root.current!.querySelector("[data-ad]");
      const scr = root.current!.querySelector("[data-screen]");
      if (prefersReducedMotion()) return;
      if (ad) gsap.set(ad, { autoAlpha: 0 });

      if (!landed.current) {
        ScrollTrigger.create({
          trigger: root.current,
          start: "top 80%",
          once: true,
          onEnter: () => {
            landed.current = true;
            adIn(ad, format, 0.15);
          },
        });
        return;
      }
      gsap.fromTo(scr, { autoAlpha: 0.4, scale: 0.985 }, { autoAlpha: 1, scale: 1, duration: 0.55, ease: "power2.out" });
      adIn(ad, format, 0.22);
    },
    { scope: root, dependencies: [niche.id, format, host, replayKey, showAd], revertOnUpdate: true, lazy: true },
  );

  return (
    <div ref={root} data-ad-manual className={className}>
      <Phone label={`${niche.ad.brand} ${format} ad shown in ${niche.app.name}`}>
        <div key={`${host}-${niche.id}-${format}-${screen}-${showAd}`} data-screen className="absolute inset-0">
          {screen === "lock" ? (
            <LockScreen niche={niche} />
          ) : (
            <AppScreen niche={niche} format={format} host={host} showAd={showAd} />
          )}
        </div>
      </Phone>
    </div>
  );
}
