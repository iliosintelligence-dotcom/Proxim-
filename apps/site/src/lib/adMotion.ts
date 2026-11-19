"use client";

import { gsap } from "./gsap";
import type { Format } from "./data";

/*
  How each format arrives says what it is.
  Banner: rises from the bottom of the app and settles, like content that
  belongs there. Notification: drops in from above with a small overshoot,
  like a real alert landing on the lock screen.
*/

export function adIn(el: Element | null, format: Format, delay = 0) {
  if (!el) return gsap.timeline();
  return format === "banner"
    ? gsap.fromTo(
        el,
        { yPercent: 115, autoAlpha: 0 },
        { yPercent: 0, autoAlpha: 1, duration: 1.05, ease: "expo.out", delay, overwrite: true },
      )
    : gsap.fromTo(
        el,
        { yPercent: -150, scale: 0.94, autoAlpha: 0 },
        { yPercent: 0, scale: 1, autoAlpha: 1, duration: 0.85, ease: "back.out(1.5)", delay, overwrite: true },
      );
}

export function adOut(el: Element | null, format: Format) {
  if (!el) return gsap.timeline();
  return gsap.to(el, {
    yPercent: format === "banner" ? 115 : -150,
    autoAlpha: 0,
    duration: 0.38,
    ease: "power2.in",
    overwrite: true,
  });
}

export function adReplay(el: Element | null, format: Format) {
  const tl = gsap.timeline();
  tl.add(adOut(el, format)).add(adIn(el, format), "+=0.12");
  return tl;
}
