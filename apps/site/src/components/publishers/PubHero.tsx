"use client";

import { Ban, Check, MapPin, Smartphone } from "lucide-react";
import type React from "react";
import { useRef, useState } from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import { nicheById, publisherEcpm, type NicheId } from "@/lib/data";
import { naira } from "@/lib/format";
import { Phone } from "@/components/phone/Phone";
import { AppScreen } from "@/components/phone/AppScreens";
import { Button } from "@/components/site/Button";
import { Frame } from "@/components/site/Frame";
import { Odometer } from "@/components/motion/Odometer";
import { Photo } from "@/components/phone/Photo";

type Event = {
  id: number;
  brand: string;
  kind: "Banner view" | "Notification";
  amount: number;
};

const rotation: NicheId[] = ["faith", "food", "education", "fitness"];

/**
 * The app owner's hero mirrors the advertiser's, seen from the other side:
 * one app (theirs) with its dashboard laid out around it. Every time an ad
 * lands in the phone, a line of earnings is written and the month's total
 * rolls up. Ads become money in front of you.
 *
 * Four cards, two either side of the phone, upright and aligned, each tied
 * to the phone by a short line. Smaller screens keep two compact cards
 * tucked against the phone's edges.
 */
export function PubHero() {
  const root = useRef<HTMLElement>(null);
  const [adIndex, setAdIndex] = useState(0);
  const [total, setTotal] = useState(482_310);
  const [events, setEvents] = useState<Event[]>([
    { id: 3, brand: "Mama Titi’s Kitchen", kind: "Banner view", amount: 41 },
    { id: 2, brand: "Grace Assembly", kind: "Notification", amount: 108 },
    { id: 1, brand: "Bright Minds Tutorial", kind: "Banner view", amount: 83 },
  ]);
  const ad = nicheById(rotation[adIndex]);

  const earn = (brand: string) => {
    // A burst of views lands at once; per-view sums are sample rates.
    const views = 40 + Math.floor(Math.random() * 120);
    const amount = (views * publisherEcpm.banner) / 1000;
    setTotal((t) => t + amount);
    setEvents((e) =>
      [
        { id: Date.now(), brand, kind: "Banner view" as const, amount },
        ...e,
      ].slice(0, 4),
    );
  };

  useGSAP(
    () => {
      const intro = gsap.utils.toArray<HTMLElement>("[data-intro]");
      if (prefersReducedMotion()) {
        gsap.set(intro, { autoAlpha: 1 });
        return;
      }
      gsap.from(intro, {
        y: 24,
        autoAlpha: 0,
        duration: 1.2,
        stagger: 0.08,
        delay: 0.3,
      });

      // The cards slide out from behind the phone into their places, then
      // breathe gently. Scrolling away, they drift apart a little.
      const cards = gsap.utils.toArray<HTMLElement>("[data-card]");
      cards.forEach((el, i) => {
        const side = el.dataset.card === "left" ? 1 : -1;
        gsap.from(el, {
          x: side * 60,
          autoAlpha: 0,
          duration: 1.1,
          ease: "expo.out",
          delay: 0.9 + (i % 2) * 0.12,
        });
        gsap.to(el.querySelector("[data-card-float]"), {
          y: i % 2 ? 5 : -5,
          duration: 2.6 + (i % 3) * 0.4,
          ease: "sine.inOut",
          yoyo: true,
          repeat: -1,
        });
        gsap.to(el, {
          x: -side * 24,
          y: -30,
          ease: "none",
          scrollTrigger: {
            trigger: el,
            start: "top 40%",
            end: "bottom top",
            scrub: 0.8,
          },
        });
      });

      const loop = gsap
        .timeline({ repeat: -1, delay: 2.6 })
        .call(() => setAdIndex((i) => (i + 1) % rotation.length))
        .to({}, { duration: 3.4 });
      ScrollTrigger.create({
        trigger: root.current,
        start: "top bottom",
        end: "bottom top",
        onToggle: (self) => (self.isActive ? loop.resume() : loop.pause()),
      });
    },
    { scope: root },
  );

  // Each new ad lands, then gets paid for.
  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const el = root.current!.querySelector("[data-ad]");
      gsap.set(el, { autoAlpha: 0 });
      adIn(el, "banner", adIndex === 0 ? 1.1 : 0.1);
      const t = gsap.delayedCall(adIndex === 0 ? 1.6 : 0.6, () =>
        earn(ad.ad.brand),
      );
      return () => t.kill();
    },
    { scope: root, dependencies: [adIndex] },
  );

  // Newest ledger line slides in on top.
  useGSAP(
    () => {
      if (prefersReducedMotion() || events[0].id < 10) return;
      const line = root.current!.querySelector("[data-ledger] li:first-child");
      if (line) gsap.from(line, { y: -12, autoAlpha: 0, duration: 0.7 });
    },
    { scope: root, dependencies: [events[0].id] },
  );

  return (
    <section
      ref={root}
      data-ad-manual
      className="relative overflow-hidden bg-paper"
    >
      <Frame className="pt-[clamp(2.75rem,6.5vw,5rem)] text-center">
        <p
          data-intro
          className="t-label inline-flex items-center gap-2 text-ink"
        >
          <Smartphone className="size-3.5 text-blue" strokeWidth={2.2} />/ For
          app owners and developers
        </p>
        <h1 data-led className="t-display mx-auto mt-6 max-w-[19ch]">
          Get paid for the attention you already have.
        </h1>
        <p data-intro className="t-lead mx-auto mt-5 max-w-[40rem]">
          Add Proxim to your app and earn from local ads that fit your users.
          You decide which brands appear and how often. Your users get offers
          from businesses near them that they actually want.
        </p>
        <div
          data-intro
          className="mx-auto mt-8 grid w-full max-w-[20rem] gap-3 sm:flex sm:w-auto sm:max-w-none sm:justify-center"
        >
          <Button href="#start" arrow>
            Start earning
          </Button>
          <Button href="#estimate" variant="secondary">
            Estimate your earnings
          </Button>
        </div>
        <p data-intro className="t-label mt-5 text-faint">
          Free to join · Paid monthly in naira · Live in an afternoon
        </p>
      </Frame>

      {/* The phone, held up from the bottom edge, with its dashboard around it. */}
      <div
        data-sticker-stage
        className="relative mx-auto mt-[clamp(2.5rem,5vw,3.5rem)] h-[clamp(30rem,52vw,40rem)] max-w-[1180px] overflow-hidden px-[var(--gutter)]"
      >
        <div className="mx-auto flex h-full max-w-[62rem] justify-center lg:grid lg:grid-cols-[1fr_17.5rem_1fr] lg:gap-x-[clamp(1.5rem,4vw,3.5rem)]">
          {/* Left column (wide screens) */}
          <div className="hidden flex-col items-end justify-end gap-5 pb-[clamp(3rem,6vw,5rem)] lg:flex">
            <Card side="left">
              <p className="t-label text-faint">Earned this month</p>
              <p className="mt-2 font-display text-[1.9rem] leading-none tracking-[-0.04em]">
                <Odometer value={naira(total)} />
              </p>
              <Bars />
              <p className="mt-3 text-[0.78rem] text-muted-foreground">
                Paid to your bank on the 1st
              </p>
            </Card>
            <Card side="left">
              <p className="t-label text-faint">Your rules · Daily Word</p>
              <ul className="mt-3 space-y-2 text-[0.84rem] font-medium">
                {rules.map((r) => (
                  <li
                    key={r.label}
                    className={`flex items-center justify-between ${r.on ? "" : "text-faint"}`}
                  >
                    {r.label}
                    {r.on ? (
                      <span className="grid size-5 place-items-center rounded-full bg-blue text-white">
                        <Check className="size-3" strokeWidth={3} />
                      </span>
                    ) : (
                      <Ban className="size-4" strokeWidth={2.2} />
                    )}
                  </li>
                ))}
              </ul>
            </Card>
          </div>

          {/* The phone */}
          <div className="relative h-full w-[min(17.5rem,64vw)] shrink-0 lg:w-auto">
            <div className="absolute inset-x-0 bottom-0 translate-y-[30%]">
              <div data-intro>
                <Phone label={`Daily Word app showing a ${ad.ad.brand} banner`}>
                  <AppScreen niche={ad} host="faith" format="banner" />
                </Phone>
              </div>

              {/* Smaller screens: two compact cards tucked against the phone */}
              <div
                data-card="left"
                className="absolute -left-[22%] top-[16%] z-10 w-[9.5rem] sm:-left-[34%] sm:w-[11rem] lg:hidden"
              >
                <div data-card-float className={`${cardBox} p-3`}>
                  <p className="text-[0.62rem] font-medium tracking-wide text-faint uppercase">
                    Earned this month
                  </p>
                  <p className="mt-1 font-display text-[1.25rem] leading-none tracking-[-0.04em]">
                    <Odometer value={naira(total)} />
                  </p>
                </div>
              </div>
              <div
                data-card="right"
                className="absolute -right-[22%] top-[34%] z-10 w-[9.5rem] sm:-right-[34%] sm:w-[11rem] lg:hidden"
              >
                <div data-card-float className={`${cardBox} p-3`}>
                  <p className="text-[0.62rem] font-medium tracking-wide text-faint uppercase">
                    Just now
                  </p>
                  <p className="mt-1 flex items-center justify-between gap-2 text-[0.78rem] font-medium">
                    <span className="truncate">{events[0].brand}</span>
                    <span className="shrink-0 text-blue">
                      +{naira(events[0].amount)}
                    </span>
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Right column (wide screens) */}
          <div className="hidden flex-col items-start justify-end gap-5 pb-[clamp(3rem,6vw,5rem)] lg:flex">
            <Card side="right">
              <div className="flex items-center justify-between">
                <p className="t-label text-faint">Live · Daily Word</p>
                <span className="relative flex size-2">
                  <span className="absolute inline-flex size-full animate-ping rounded-full bg-blue opacity-60" />
                  <span className="relative inline-flex size-2 rounded-full bg-blue" />
                </span>
              </div>
              <ul
                data-ledger
                className="mt-2 divide-y"
                style={{ borderColor: "var(--line)" }}
              >
                {events.slice(0, 3).map((e) => (
                  <li
                    key={e.id}
                    className="flex items-center justify-between gap-3 py-2 text-[0.82rem]"
                    style={{ borderColor: "var(--line)" }}
                  >
                    <span className="truncate font-medium">{e.brand}</span>
                    <span className="shrink-0 font-medium text-blue">
                      +{naira(e.amount)}
                    </span>
                  </li>
                ))}
              </ul>
            </Card>
            <Card side="right">
              <p className="t-label text-faint">Now showing</p>
              <div className="mt-3 flex items-center gap-3">
                <span className="block size-11 shrink-0 overflow-hidden rounded-full">
                  <Photo id={ad.ad.photo} w={88} h={88} />
                </span>
                <span className="min-w-0 leading-tight">
                  <span className="block truncate text-[0.9rem] font-semibold">
                    {ad.ad.brand}
                  </span>
                  <span className="mt-1 flex items-center gap-1 text-[0.78rem] text-muted-foreground">
                    <MapPin className="size-3" /> Banner · {ad.ad.distance} away
                  </span>
                </span>
              </div>
            </Card>
          </div>
        </div>
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-paper to-transparent" />
      </div>
    </section>
  );
}

const rules = [
  { label: "Food and drink", on: true },
  { label: "Churches and events", on: true },
  { label: "Education", on: true },
  { label: "Betting", on: false },
];

const cardBox =
  "rounded-2xl bg-white text-left shadow-[0_18px_40px_-20px_rgb(13_13_13/0.35)] ring-1 ring-black/5";

/** One dashboard card beside the phone, tied to it by a short dotted line. */
function Card({
  side,
  children,
}: {
  side: "left" | "right";
  children: React.ReactNode;
}) {
  return (
    <div data-card={side} className="relative w-[16rem]">
      <span
        aria-hidden="true"
        className={`absolute top-1/2 flex w-[clamp(1.5rem,4vw,3.5rem)] items-center ${side === "left" ? "left-full" : "right-full flex-row-reverse"}`}
      >
        <span className="h-px flex-1 border-t border-dashed border-ink/25" />
        <span className="size-1.5 rounded-full bg-blue" />
      </span>
      <div data-card-float className={`${cardBox} p-4`}>
        {children}
      </div>
    </div>
  );
}

function Bars() {
  return (
    <div className="mt-4 flex h-10 items-end gap-[3px]" aria-hidden="true">
      {Array.from({ length: 20 }, (_, i) => (
        <span
          key={i}
          className={`flex-1 rounded-[1px] ${i === 19 ? "bg-blue" : "bg-stone-2"}`}
          style={{ height: `${30 + ((i * 37) % 50) + i}%` }}
        />
      ))}
    </div>
  );
}
