"use client";

import { useRef, type ReactNode } from "react";
import { gsap, SplitText, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Frame } from "./Frame";
import { Button } from "./Button";
import { LogoMark } from "./Logo";
import { AdPosterWall } from "./AdPosters";

/**
 * The finale, shared by every page (Proxim's take on Ledger's ending).
 * Everything happens on one pinned screen:
 *
 * 1. A giant statement slides past as you scroll.
 * 2. Pip appears, rings pulse out of it, and the radius grows until it
 *    fills the screen in blue: proximity is the last thing you see.
 * 3. The moment the blue is full, the call to action and the poster wall
 *    rise in on it. No extra scrolling to reach them.
 */
export function FinalCta({
  id,
  title,
  lead,
  primary,
  secondary,
  statement = "Every business deserves to be found nearby",
  pip = "Ready when you are.",
}: {
  id?: string;
  title: string;
  lead: string;
  primary: { href: string; label: string };
  secondary?: { href: string; label: string };
  /** Kept for compatibility; the finale now shows the poster wall. */
  cards?: ReactNode[];
  statement?: string;
  pip?: string;
}) {
  const root = useRef<HTMLElement>(null);

  useGSAP(
    () => {
      const q = gsap.utils.selector(root);
      if (prefersReducedMotion()) {
        gsap.set(q("[data-ring-fill]"), { clipPath: "circle(150% at 50% 50%)" });
        gsap.set(q("[data-cta]"), { autoAlpha: 1 });
        gsap.set(q("[data-statement-wrap]"), { autoAlpha: 0 });
        return;
      }
      const text = q("[data-statement]")[0] as HTMLElement;
      gsap.set(q("[data-cta]"), { autoAlpha: 0 });
      // Headline rises word by word from behind a mask; the rest follows.
      const words = SplitText.create(q("[data-cta-title]")[0], { type: "words", mask: "words", aria: "auto" }).words;
      gsap.set(words, { yPercent: 110 });
      gsap.set(q("[data-cta-in]"), { y: 36, autoAlpha: 0 });
      // Each poster column enters from the end it ticks toward.
      gsap.set(q('[data-col="down"]'), { yPercent: -35, autoAlpha: 0 });
      gsap.set(q('[data-col="up"]'), { yPercent: 35, autoAlpha: 0 });

      const tl = gsap.timeline({
        scrollTrigger: {
          trigger: q("[data-finale-pin]")[0],
          start: "top top",
          end: "+=200%",
          pin: true,
          scrub: 0.5,
        },
      });
      tl.fromTo(
        text,
        { x: () => window.innerWidth * 0.9 },
        { x: () => -(text.scrollWidth - window.innerWidth * 0.35), ease: "none", duration: 2.2 },
      )
        .fromTo(q("[data-pin-mark]"), { scale: 0, opacity: 0 }, { scale: 1, opacity: 1, duration: 0.35, ease: "back.out(2)" }, 1.9)
        .fromTo(
          q("[data-ring-line]"),
          { scale: 0.2, opacity: 0.9 },
          { scale: 1.6, opacity: 0, duration: 0.6, stagger: 0.12, ease: "power1.out" },
          2.05,
        )
        .fromTo(
          q("[data-ring-fill]"),
          { clipPath: "circle(0% at 50% 50%)" },
          { clipPath: "circle(150% at 50% 50%)", duration: 0.9, ease: "power2.in" },
          2.3,
        )
        .to(q("[data-pin-mark]"), { opacity: 0, scale: 0.6, duration: 0.2 }, 3.05)
        // The instant the blue is full, the call to action is on it.
        .set(q("[data-cta]"), { autoAlpha: 1 }, 3.2)
        .to(q("[data-cta-in]"), { y: 0, autoAlpha: 1, duration: 0.45, stagger: 0.08, ease: "power3.out" }, 3.2)
        .to(words, { yPercent: 0, duration: 0.5, stagger: 0.035, ease: "power3.out" }, 3.25)
        .to(q('[data-col="down"]'), { yPercent: 0, autoAlpha: 1, duration: 0.6, ease: "power3.out" }, 3.25)
        .to(q('[data-col="up"]'), { yPercent: 0, autoAlpha: 1, duration: 0.6, ease: "power3.out" }, 3.32)
        .to({}, { duration: 0.6 });
    },
    { scope: root },
  );

  return (
    <section id={id} ref={root} data-pip={pip} className="relative scroll-mt-16 text-white">
      <div data-finale-pin className="relative h-screen min-h-[40rem] overflow-hidden bg-night">
        {/* 1: statement */}
        <div data-statement-wrap className="absolute inset-0 flex items-center">
          <p data-statement className="whitespace-nowrap font-display text-[clamp(4.5rem,15vw,13rem)] leading-none tracking-[-0.04em]">
            {statement}
          </p>
        </div>

        {/* 2: rings, then the radius fills the screen */}
        <div className="pointer-events-none absolute inset-0 grid place-items-center">
          {[0, 1, 2].map((i) => (
            <span key={i} data-ring-line className="absolute size-[38vmin] rounded-full border-2 border-blue opacity-0" />
          ))}
        </div>
        <div data-ring-fill className="absolute inset-0 bg-blue [clip-path:circle(0%_at_50%_50%)]" />
        <div data-pin-mark className="pointer-events-none absolute inset-0 grid place-items-center opacity-0">
          <span className="grid size-24 place-items-center rounded-full bg-white shadow-[0_20px_60px_-10px_rgb(0_0_0/0.5)]">
            <LogoMark className="h-12 w-auto" />
          </span>
        </div>

        {/* 3: the call to action, on the blue the radius left behind */}
        <div data-cta className="invisible absolute inset-0 flex items-center">
          <Frame night className="grid w-full items-center gap-8 py-20 lg:grid-cols-[1fr_1fr] lg:gap-14 lg:border-x-0">
            <div>
              <p data-cta-in className="t-label mb-5 text-white/80">
                / Get started
              </p>
              <h2 data-cta-title className="t-display max-w-[13ch] !text-[clamp(2.3rem,5.4vw,4.5rem)]">
                {title}
              </h2>
              <p data-cta-in className="mt-5 max-w-[30rem] text-[1rem] leading-relaxed text-white/75 sm:text-[1.06rem]">
                {lead}
              </p>
              <div data-cta-in className="mt-7 flex flex-wrap gap-3">
                <Button href={primary.href} variant="night" arrow>
                  {primary.label}
                </Button>
                {secondary && (
                  <Button href={secondary.href} variant="primary">
                    {secondary.label}
                  </Button>
                )}
              </div>
            </div>
            <div data-wall data-ad-manual className="relative h-[34vh] lg:h-[min(40rem,78vh)]">
              <AdPosterWall className="h-full" />
            </div>
          </Frame>
        </div>
      </div>
    </section>
  );
}
