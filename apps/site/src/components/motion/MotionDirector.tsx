"use client";

import { useEffect, useRef } from "react";
import { usePathname } from "next/navigation";
import Lenis from "lenis";
import { gsap, ScrollTrigger, SplitText, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import type { Format } from "@/lib/data";
import { setLenis } from "@/lib/lenis";

/**
 * One place that owns page-wide motion, so every page moves the same way.
 *
 * [data-led]    Headlines rise into place line by line from behind a mask,
 *               the way Ledger's do: confident, quick, and done once.
 * [data-reveal] Supporting copy and cards rise 18px and fade in, once.
 *               Quiet on purpose, so the headline stays the event.
 */
export function MotionDirector() {
  const pathname = usePathname();
  const lenisRef = useRef<Lenis | null>(null);

  // Smooth scroll: created once for the whole session.
  useEffect(() => {
    if (prefersReducedMotion()) return;
    const lenis = new Lenis({ lerp: 0.11, wheelMultiplier: 1 });
    lenisRef.current = lenis;
    setLenis(lenis);
    lenis.on("scroll", ScrollTrigger.update);
    const tick = (time: number) => lenis.raf(time * 1000);
    gsap.ticker.add(tick);
    gsap.ticker.lagSmoothing(0);
    return () => {
      gsap.ticker.remove(tick);
      lenis.destroy();
      lenisRef.current = null;
      setLenis(null);
    };
  }, []);

  // Per-page animations: rebuilt whenever the route changes.
  useEffect(() => {
    if (!window.location.hash) lenisRef.current?.scrollTo(0, { immediate: true });
    const reduce = prefersReducedMotion();

    const ctx = gsap.context(() => {
      const leds = gsap.utils.toArray<HTMLElement>("[data-led]");
      const reveals = gsap.utils.toArray<HTMLElement>("[data-reveal]");

      if (reduce) {
        gsap.set([...leds, ...reveals], { autoAlpha: 1 });
        return;
      }

      // Dark sections switch on like a screen: they arrive as an inset,
      // rounded panel and open to full bleed as they reach the reader.
      gsap.utils.toArray<HTMLElement>("[data-screen]").forEach((sec) => {
        gsap.fromTo(
          sec,
          { clipPath: "inset(0% 3.5% 0% 3.5% round 28px)" },
          {
            clipPath: "inset(0% 0% 0% 0% round 0px)",
            ease: "none",
            scrollTrigger: { trigger: sec, start: "top bottom", end: "top 35%", scrub: 0.6 },
          },
        );
      });

      // Phones float a little slower than the page, so they sit in front.
      gsap.utils.toArray<HTMLElement>("[data-float]").forEach((el) => {
        gsap.fromTo(
          el,
          { y: 50 },
          { y: -50, ease: "none", scrollTrigger: { trigger: el, start: "top bottom", end: "bottom top", scrub: 0.8 } },
        );
      });

      // Photo sections: the photo starts slightly enlarged and settles as
      // you pass, like a camera pulling focus.
      gsap.utils.toArray<HTMLElement>("[data-band-photo]").forEach((ph) => {
        gsap.fromTo(
          ph,
          { scale: 1.18, yPercent: -4 },
          {
            scale: 1.02,
            yPercent: 4,
            ease: "none",
            scrollTrigger: { trigger: ph.parentElement, start: "top bottom", end: "bottom top", scrub: 0.8 },
          },
        );
      });

      leds.forEach((el) => {
        const icons = el.querySelectorAll("[data-hicon]");
        gsap.set(el, { autoAlpha: 1 });
        // Re-splits itself once the display font has loaded or the width
        // changes, so line breaks are always the real ones.
        SplitText.create(el, {
          type: "lines",
          mask: "lines",
          autoSplit: true,
          aria: "auto",
          onSplit: (self) =>
            gsap.from(self.lines, {
              yPercent: 105,
              duration: 1.1,
              ease: "expo.out",
              stagger: 0.09,
              scrollTrigger: { trigger: el, start: "top 88%", once: true },
            }),
        });
        if (icons.length) {
          gsap.set(icons, { opacity: 0, scale: 0.4 });
          gsap.to(icons, {
            opacity: 1,
            scale: 1,
            duration: 0.8,
            ease: "back.out(2)",
            delay: 0.5,
            stagger: 0.1,
            scrollTrigger: { trigger: el, start: "top 88%", once: true },
          });
        }
      });

      // Ads never just sit on the page: they land when their phone scrolls
      // into view. Sections that choreograph their own ads opt out with
      // [data-ad-manual].
      gsap.utils
        .toArray<HTMLElement>("[data-ad]")
        .filter((el) => !el.closest("[data-ad-manual]"))
        .forEach((el) => {
          ScrollTrigger.create({
            trigger: el.parentElement ?? el,
            start: "top 85%",
            once: true,
            onEnter: () => adIn(el, el.dataset.ad as Format, 0.15),
          });
        });

      gsap.set(reveals, { autoAlpha: 0, y: 18 });
      ScrollTrigger.batch(reveals, {
        start: "top 90%",
        once: true,
        onEnter: (batch) =>
          gsap.to(batch, {
            autoAlpha: 1,
            y: 0,
            duration: 1,
            ease: "expo.out",
            stagger: 0.07,
            delay: 0.08,
          }),
      });
    });

    // Fonts change line lengths; re-measure trigger points once they're in.
    document.fonts?.ready.then(() => ScrollTrigger.refresh());

    return () => ctx.revert();
  }, [pathname]);

  return null;
}
