"use client";

import { useRef } from "react";
import { CalendarDays, ChartLine, Crosshair, Radar, Smartphone, type LucideIcon } from "lucide-react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Photo } from "@/components/phone/Photo";

type Card = { icon: LucideIcon; title: string; body: string; bg: string; photo: string };

const cards: Card[] = [
  {
    icon: Crosshair,
    title: "Niche",
    body: "Your ad only runs in apps that match what you sell. Church invites in scripture apps, gym passes in step counters.",
    bg: "bg-[linear-gradient(160deg,#2f6bff,#0040d6)]",
    photo: "1609532587710-b00027c0594b",
  },
  {
    icon: Radar,
    title: "Radius",
    body: "Draw a circle around your shop, from 2 km to your whole state. Phones outside it never see you, and you never pay for them.",
    bg: "bg-[linear-gradient(160deg,#1a1d2b,#05060a)]",
    photo: "1687422808191-93810cd07ab0",
  },
  {
    icon: Smartphone,
    title: "Format",
    body: "A banner inside the app while they scroll, or a notification on the lock screen when they’re near you.",
    bg: "bg-[linear-gradient(160deg,#4a4f6b,#23263a)]",
    photo: "1687422808248-f807f4ea2a2e",
  },
  {
    icon: CalendarDays,
    title: "Schedule",
    body: "Run it for a weekend sale or a whole season. Views are spread evenly so day one doesn’t eat day ten’s budget.",
    bg: "bg-[linear-gradient(160deg,#ffe36e,#ffd02a)] !text-ink",
    photo: "1709837167686-a2e33aad1bf0",
  },
  {
    icon: ChartLine,
    title: "Live results",
    body: "Every view, tap and naira counted as it happens, by app and by street. Change anything while it runs.",
    bg: "bg-[linear-gradient(160deg,#0d0d0d,#262626)]",
    photo: "1761370571806-886404629697",
  },
];

const tilt = [-7, 5, -3, 6, -2];

/**
 * Adapted from Ledger's stacking cards. The heading holds still while the
 * five parts of a campaign arrive one at a time and settle on the pile,
 * each slightly askew like cards dealt onto a table. By the end you've
 * been handed the whole product.
 */
export function CardStack() {
  const root = useRef<HTMLElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const mm = gsap.matchMedia();
      mm.add("(min-width: 1024px)", () => {
        const items = gsap.utils.toArray<HTMLElement>("[data-stack-card]");
        gsap.set(items, { yPercent: 160, rotate: (i) => tilt[i] * 2, opacity: 0 });
        gsap.set(items[0], { yPercent: 0, rotate: tilt[0], opacity: 1 });
        const tl = gsap.timeline({
          scrollTrigger: {
            trigger: root.current,
            start: "top top",
            end: `+=${items.length * 70}%`,
            pin: "[data-stack-pin]",
            scrub: 0.8,
          },
        });
        items.slice(1).forEach((card, i) => {
          tl.to(card, { yPercent: 0, rotate: tilt[i + 1], opacity: 1, duration: 1, ease: "power2.out" }, i);
          // The cards underneath sink a little, so the pile reads as depth.
          tl.to(items.slice(0, i + 1), { scale: (j) => 1 - (i + 1 - j) * 0.04, y: (j) => -(i + 1 - j) * 14, duration: 1, ease: "power2.out" }, i);
        });
        tl.to({}, { duration: 0.4 });
      });
      return () => mm.revert();
    },
    { scope: root },
  );

  return (
    <section ref={root} data-pip="Five settings. That’s the whole campaign." className="relative bg-paper">
      <div className="border-t" style={{ borderColor: "var(--line)" }}>
        <div data-stack-pin className="relative mx-auto max-w-[1240px] px-[var(--gutter)] lg:h-screen lg:border-x" style={{ borderColor: "var(--line)" }}>
          <div className="pointer-events-none absolute inset-x-[var(--gutter)] bottom-6 top-24 hidden justify-between lg:flex">
            <span className="t-label self-start text-ink">/ One campaign</span>
            <span className="t-label self-end text-ink">/ Proxim</span>
            <span className="t-label self-end text-ink">/ Ads platform</span>
          </div>

          <div className="grid h-full items-center gap-12 py-[clamp(4.5rem,10vw,8rem)] lg:grid-cols-[1fr_1fr] lg:py-0">
            <div className="relative order-2 mx-auto w-full max-w-[28rem] lg:order-1 lg:h-[min(34rem,68vh)] lg:max-w-[min(36rem,100%)]">
              {cards.map((c, i) => (
                <article
                  key={c.title}
                  data-stack-card
                  className={`mb-4 flex flex-col overflow-hidden rounded-2xl p-6 text-white shadow-[0_30px_60px_-28px_rgb(13_13_13/0.55)] lg:absolute lg:inset-0 lg:mb-0 lg:rounded-3xl lg:p-10 ${c.bg}`}
                  style={{ zIndex: i + 1 }}
                >
                  <div className="flex items-start justify-between">
                    <span className="grid size-9 place-items-center rounded-full bg-white/15 backdrop-blur lg:size-12">
                      <c.icon className="size-[18px] lg:size-6" strokeWidth={2} />
                    </span>
                    <span className="block size-12 overflow-hidden rounded-full ring-2 ring-white/40 lg:size-24 lg:ring-4">
                      <Photo id={c.photo} w={192} h={192} />
                    </span>
                  </div>
                  <h3 className="mt-14 font-display text-[1.9rem] leading-none tracking-[-0.03em] lg:mt-auto lg:text-[clamp(2.6rem,4vw,3.6rem)]">{c.title}</h3>
                  <p className="mt-3 text-[0.92rem] leading-relaxed opacity-80 lg:mt-5 lg:max-w-[30ch] lg:text-[1.1rem]">{c.body}</p>
                </article>
              ))}
            </div>
            <div className="order-1 lg:order-2 lg:text-right">
              <p className="t-label mb-6 text-ink lg:hidden">/ One campaign</p>
              <h2 data-led className="t-section lg:ml-auto lg:max-w-[11ch]">
                Everything a campaign needs, in one place
              </h2>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
