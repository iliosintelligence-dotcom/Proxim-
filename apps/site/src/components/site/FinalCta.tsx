"use client";

import { useRef, type ReactNode } from "react";
import { gsap, ScrollTrigger, SplitText, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Frame } from "./Frame";
import { Button } from "./Button";
import { LogoMark } from "./Logo";
import { AdPosterWall } from "./AdPosters";
import { HIcon } from "./HIcon";

/**
 * The finale, shared by every page (Proxim's take on Ledger's ending).
 * Everything happens on one pinned screen:
 *
 * 1. A big statement rises in line by line, like every headline on the
 *    site, and its photo pills pop in after it.
 * 2. Pip sits beside the last word. Scrolling on, rings pulse out of
 *    that Pip and its radius grows until it fills the screen in blue:
 *    proximity is the last thing you see.
 * 3. The moment the blue is full, the call to action and the poster wall
 *    rise in on it. No extra scrolling to reach them.
 */
export function FinalCta({
  id,
  title,
  lead,
  primary,
  secondary,
  statement = "Every business {0} deserves to be found {1} nearby",
  photos = ["1698827623494-c4e1196ba0ca", "1687422808191-93810cd07ab0"],
  pip = "Ready when you are.",
}: {
  id?: string;
  title: string;
  lead: string;
  primary: { href: string; label: string };
  secondary?: { href: string; label: string };
  /** Kept for compatibility; the finale now shows the poster wall. */
  cards?: ReactNode[];
  /** Write {0}, {1}… where each photo pill from `photos` goes. */
  statement?: string;
  photos?: string[];
  pip?: string;
}) {
  const root = useRef<HTMLElement>(null);
  // Photo pills go where the {n} markers are; the last word is set in
  // blue, the colour that fills the screen next.
  const parts = statement.split(/\{(\d+)\}/);
  const tail = parts.pop() ?? "";
  const cut = tail.lastIndexOf(" ") + 1;

  useGSAP(
    () => {
      const q = gsap.utils.selector(root);
      if (prefersReducedMotion()) {
        gsap.set(q("[data-ring-fill]"), { clipPath: "circle(150% at 50% 50%)" });
        gsap.set(q("[data-cta]"), { autoAlpha: 1 });
        gsap.set(q("[data-statement-wrap]"), { autoAlpha: 0 });
        return;
      }
      const pin = q("[data-finale-pin]")[0];

      // 1. Once the finale is nearly on screen, the statement rises into
      //    place line by line from behind a mask, the same reveal as every
      //    headline on the site, then its photo pills pop in.
      //    It plays on its own, not tied to the scrollbar.
      const statementEl = q("[data-statement]")[0] as HTMLElement;
      // Re-splitting rebuilds the pills, so they are looked up fresh each time.
      // Pip beside the last word pops in with them.
      const pills = () => statementEl.querySelectorAll("[data-hicon], [data-pip-inline]");
      const inlinePip = () => statementEl.querySelector<HTMLElement>("[data-pip-inline]")!;
      gsap.set(q("[data-statement-label]"), { y: 16, autoAlpha: 0 });
      // Re-splits itself once the display font has loaded or the width
      // changes; after the reveal has played, new lines simply stay put.
      let revealed = false;
      let lines: Element[] = [];
      SplitText.create(statementEl, {
        type: "lines",
        mask: "lines",
        autoSplit: true,
        aria: "auto",
        onSplit: (self) => {
          lines = self.lines;
          if (!revealed) gsap.set(lines, { yPercent: 105 });
          if (!revealed) gsap.set(pills(), { opacity: 0, scale: 0.4 });
          gsap.set(statementEl, { autoAlpha: 1 });
        },
      });
      ScrollTrigger.create({
        trigger: pin,
        start: "top 20%",
        once: true,
        onEnter: () => {
          revealed = true;
          gsap
            .timeline()
            .to(q("[data-statement-label]"), { y: 0, autoAlpha: 1, duration: 0.6, ease: "power3.out" }, 0)
            .to(lines, { yPercent: 0, duration: 1.1, ease: "expo.out", stagger: 0.09 }, 0.05)
            .to(pills(), { opacity: 1, scale: 1, duration: 0.8, ease: "back.out(2)", stagger: 0.1 }, 0.5);
        },
      });

      // Everything after it waits, hidden, for the scroll.
      gsap.set(q("[data-ring-line]"), { autoAlpha: 0, scale: 1 });
      gsap.set(q("[data-pin-mark]"), { autoAlpha: 0, xPercent: -50, yPercent: -50 });
      gsap.set(q("[data-cta]"), { autoAlpha: 0 });
      const words = SplitText.create(q("[data-cta-title]")[0], { type: "words", mask: "words", aria: "auto" }).words;
      gsap.set(words, { yPercent: 110 });
      gsap.set(q("[data-cta-in]"), { y: 36, autoAlpha: 0 });
      // Each poster column enters from the end it ticks toward.
      gsap.set(q('[data-col="down"]'), { yPercent: -35, autoAlpha: 0 });
      gsap.set(q('[data-col="up"]'), { yPercent: 35, autoAlpha: 0 });

      // Where Pip sits on the pinned screen. Offsets ignore transforms, so
      // this is its resting place even mid-reveal.
      const pipAt = () => {
        let el: HTMLElement | null = inlinePip();
        const size = el.offsetWidth;
        let x = size / 2;
        let y = el.offsetHeight / 2;
        while (el && el !== pin) {
          x += el.offsetLeft;
          y += el.offsetTop;
          el = el.offsetParent as HTMLElement | null;
        }
        const w = (pin as HTMLElement).offsetWidth;
        const h = (pin as HTMLElement).offsetHeight;
        return { x, y, size, r: Math.hypot(Math.max(x, w - x), Math.max(y, h - y)) + 20 };
      };
      const at = (key: "x" | "y" | "size") => () => pipAt()[key];

      // 2. Scrolling on, a twin of Pip takes over from the one in the text
      //    (same place, same size) so it can stay on top of the blue. It
      //    pings, the rings pulse out of it and its radius fills the screen.
      // 3. The instant the blue is full, the call to action is on it.
      gsap
        .timeline({
          scrollTrigger: { trigger: pin, start: "top top", end: "+=140%", pin: true, scrub: 0.5, invalidateOnRefresh: true },
        })
        .set(q("[data-pin-mark], [data-rings]"), { left: at("x"), top: at("y") }, 0)
        .set(q("[data-pin-mark], [data-ring-line]"), { width: at("size"), height: at("size") }, 0)
        .set(q("[data-pin-mark]"), { autoAlpha: 1, scale: 1 }, 0.02)
        // Hidden through a variable: re-splitting rebuilds the Pip in the text.
        .fromTo(statementEl, { "--pip-v": "visible" }, { "--pip-v": "hidden", duration: 0.001, immediateRender: false }, 0.02)
        .to({}, { duration: 0.2 })
        .to(q("[data-pin-mark]"), { keyframes: [{ scale: 1.25, duration: 0.12, ease: "power2.out" }, { scale: 1, duration: 0.2, ease: "back.out(3)" }] }, 0.2)
        .to(q("[data-ring-line]"), { keyframes: [{ autoAlpha: 0.9, duration: 0.01 }, { scale: 7, autoAlpha: 0, duration: 0.6, ease: "power1.out" }], stagger: 0.1 }, 0.3)
        .fromTo(
          q("[data-ring-fill]"),
          { clipPath: () => { const p = pipAt(); return `circle(0px at ${p.x}px ${p.y}px)`; } },
          { clipPath: () => { const p = pipAt(); return `circle(${p.r}px at ${p.x}px ${p.y}px)`; }, duration: 0.8, ease: "power2.in", immediateRender: false },
          0.5,
        )
        .to(q("[data-pin-mark]"), { autoAlpha: 0, scale: 0.6, duration: 0.2 }, 1.3)
        .set(q("[data-cta]"), { autoAlpha: 1 }, 1.4)
        .to(q("[data-cta-in]"), { y: 0, autoAlpha: 1, duration: 0.45, stagger: 0.08, ease: "power3.out" }, 1.4)
        .to(words, { yPercent: 0, duration: 0.5, stagger: 0.035, ease: "power3.out" }, 1.45)
        .to(q('[data-col="down"]'), { yPercent: 0, autoAlpha: 1, duration: 0.6, ease: "power3.out" }, 1.45)
        .to(q('[data-col="up"]'), { yPercent: 0, autoAlpha: 1, duration: 0.6, ease: "power3.out" }, 1.52)
        .to({}, { duration: 0.4 });
    },
    { scope: root },
  );

  return (
    <section id={id} ref={root} data-pip={pip} className="relative scroll-mt-16 text-white">
      <div data-finale-pin className="relative h-screen min-h-[40rem] overflow-hidden bg-night">
        {/* 1: statement */}
        <div data-statement-wrap className="absolute inset-0 grid place-items-center px-6">
          <div className="text-center">
            <p data-statement-label className="t-label mb-6 text-white/60">
              / Proxim
            </p>
            <p data-statement className="invisible mx-auto max-w-[14ch] font-display text-[clamp(2.8rem,8.5vw,8rem)] leading-[0.95] tracking-[-0.04em] text-balance">
              {parts.map((part, i) =>
                i % 2 ? <HIcon key={i} photo={photos[Number(part)]} /> : part,
              )}
              {tail.slice(0, cut)}
              <span className="whitespace-nowrap">
                <span className="text-blue">{tail.slice(cut)}</span>
                <span
                  data-pip-inline
                  aria-hidden
                  className="ml-[0.14em] inline-grid [visibility:var(--pip-v,visible)] size-[0.8em] translate-y-[0.02em] place-items-center rounded-full bg-white align-baseline shadow-[0_0.05em_0.25em_rgb(0_0_0/0.35)]"
                >
                  <LogoMark className="h-[60%] w-auto" />
                </span>
              </span>
            </p>
          </div>
        </div>

        {/* 2: rings out of Pip, then its radius fills the screen. Placed on
            Pip by the timeline. */}
        <div data-rings className="pointer-events-none absolute left-1/2 top-1/2">
          {[0, 1, 2].map((i) => (
            <span key={i} data-ring-line className="absolute left-0 top-0 size-24 -translate-1/2 rounded-full border-2 border-blue opacity-0" />
          ))}
        </div>
        <div data-ring-fill className="absolute inset-0 bg-blue [clip-path:circle(0%_at_50%_50%)]" />
        <span
          data-pin-mark
          className="pointer-events-none absolute left-1/2 top-1/2 grid size-24 place-items-center rounded-full bg-white opacity-0 shadow-[0_20px_60px_-10px_rgb(0_0_0/0.5)]"
        >
          <LogoMark className="h-[60%] w-auto" />
        </span>

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
