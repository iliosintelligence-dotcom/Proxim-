"use client";

import { Check } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type React from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import { nicheById, type NicheId } from "@/lib/data";
import { Phone } from "@/components/phone/Phone";
import { LockScreen } from "@/components/phone/AppScreens";
import { NotificationAd } from "@/components/phone/Ads";
import { TriangleArrow } from "@/components/site/Button";
import { NicheIcon } from "@/components/site/NicheIcon";

const nicheChoices: NicheId[] = ["faith", "food", "fitness", "music"];
const radiusSteps = [2, 5, 10, 25];
const phonesAt = [1140, 4560, 9120, 18240];
const dayChoices = [3, 10, 30];

/**
 * The Features page in one loop: a cursor sets the three things Proxim
 * asks for (niche, radius, how long), presses launch, and the ad lands on
 * a phone nearby. It plays while it's on screen and settles on the
 * finished campaign for anyone who prefers less motion.
 */
export function BuilderDemo() {
  const root = useRef<HTMLDivElement>(null);
  const [niche, setNiche] = useState<NicheId | null>(null);
  const [radius, setRadius] = useState(0);
  const [days, setDays] = useState<number | null>(null);
  const [live, setLive] = useState(false);

  useGSAP(
    () => {
      const stage = root.current!;
      const cursor = stage.querySelector<HTMLElement>("[data-cursor]")!;
      const tap = stage.querySelector<HTMLElement>("[data-tap]")!;

      if (prefersReducedMotion()) {
        setNiche("food");
        setRadius(1);
        setDays(10);
        setLive(true);
        gsap.set(cursor, { autoAlpha: 0 });
        return;
      }

      // Where a target sits inside the stage, measured fresh on each loop.
      const at = (sel: string, fx = 0.5, fy = 0.5) => {
        const el = stage.querySelector<HTMLElement>(sel)!;
        const s = stage.getBoundingClientRect();
        const r = el.getBoundingClientRect();
        return { x: r.left - s.left + r.width * fx, y: r.top - s.top + r.height * fy };
      };
      const moveTo = (sel: string, fx?: number, fy?: number) => ({
        x: () => at(sel, fx, fy).x,
        y: () => at(sel, fx, fy).y,
        duration: 0.85,
        ease: "power3.inOut",
      });
      const click = () =>
        gsap
          .timeline()
          .to(cursor, { scale: 0.86, duration: 0.09, ease: "power2.out" })
          .to(cursor, { scale: 1, duration: 0.2, ease: "back.out(3)" })
          .fromTo(tap, { scale: 0.2, autoAlpha: 0.7 }, { scale: 1, autoAlpha: 0, duration: 0.45, ease: "power2.out" }, 0);

      const drag = { v: 0 };
      const tl = gsap.timeline({ repeat: -1, paused: true, repeatRefresh: true, defaults: { overwrite: "auto" } });
      tl.call(() => {
        setNiche(null);
        setRadius(0);
        setDays(null);
        setLive(false);
        drag.v = 0;
      })
        .set(cursor, { x: () => stage.offsetWidth * 0.62, y: () => stage.offsetHeight + 30, autoAlpha: 0, scale: 1 })
        .to(cursor, { autoAlpha: 1, duration: 0.3 })
        // 1. Niche
        .to(cursor, moveTo('[data-chip="food"]'))
        .add(click())
        .call(() => setNiche("food"), [], "<0.05")
        // 2. Radius: grab the knob and drag it to 5 km
        .to(cursor, { ...moveTo("[data-knob]"), duration: 0.7 }, "+=0.35")
        .add(click())
        .to(cursor, { x: () => at("[data-track]", 1 / 3).x, duration: 0.7, ease: "power2.inOut" })
        .to(drag, { v: 1, duration: 0.7, ease: "power2.inOut", onUpdate: () => setRadius(Math.round(drag.v)) }, "<")
        // 3. How long
        .to(cursor, moveTo('[data-days="10"]'), "+=0.3")
        .add(click())
        .call(() => setDays(10), [], "<0.05")
        // 4. Launch
        .to(cursor, moveTo("[data-launch]", 0.4), "+=0.35")
        .add(click())
        .call(() => setLive(true), [], "<0.05")
        // Let it sit, then step away
        .to(cursor, { x: "+=60", y: "+=50", autoAlpha: 0, duration: 0.6, ease: "power2.in" }, "+=1.2")
        .to({}, { duration: 2.2 });

      ScrollTrigger.create({
        trigger: stage,
        start: "top 85%",
        end: "bottom top",
        onToggle: (s) => (s.isActive ? tl.play() : tl.pause()),
      });
      const onResize = () => tl.invalidate();
      window.addEventListener("resize", onResize);
      return () => window.removeEventListener("resize", onResize);
    },
    { scope: root },
  );

  // The ad lands the moment the campaign goes live: on the phone on wide
  // screens, as the notification itself under the card on small ones.
  useEffect(() => {
    root.current?.querySelectorAll("[data-ad]").forEach((ad) => {
      if (live) adIn(ad, "notification", 0.25);
      else gsap.set(ad, { autoAlpha: 0 });
    });
  }, [live]);

  const chosen = nicheById(niche ?? "food");
  const fill = (radius / (radiusSteps.length - 1)) * 100;

  return (
    <div ref={root} data-ad-manual className="relative mx-auto mt-14 max-w-[56rem] text-left">
      <div className="grid items-center justify-items-center gap-8 lg:grid-cols-[30rem_auto] lg:justify-center lg:gap-16">
        {/* The campaign being built */}
        <div className="w-full max-w-[30rem] rounded-3xl bg-white p-5 shadow-[0_30px_60px_-34px_rgb(13_13_13/0.45)] ring-1 ring-black/5 sm:p-6">
          <div className="flex items-center justify-between">
            <p className="font-display text-[1.25rem] tracking-[-0.02em]">New campaign</p>
            <span
              className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[0.72rem] font-semibold transition-colors duration-300 ${
                live ? "bg-[#e3f7e8] text-[#13804b]" : "bg-stone text-muted-foreground"
              }`}
            >
              <span className={`size-1.5 rounded-full ${live ? "bg-[#1fa463]" : "bg-faint"}`} />
              {live ? "Live" : "Draft"}
            </span>
          </div>

          <Row n={1} label="Niche" done={!!niche}>
            <div className="flex flex-wrap gap-2">
              {nicheChoices.map((id) => {
                const on = niche === id;
                return (
                  <span
                    key={id}
                    data-chip={id}
                    className={`inline-flex items-center gap-1.5 rounded-full py-1 pl-1 pr-3 text-[0.8rem] font-medium ring-1 transition-colors duration-300 ${
                      on ? "bg-ink text-white ring-ink" : "bg-white text-ink ring-black/10"
                    }`}
                  >
                    <NicheIcon niche={nicheById(id)} size="sm" className="!size-6" />
                    {nicheById(id).label}
                  </span>
                );
              })}
            </div>
          </Row>

          <Row n={2} label="Radius" done={radius > 0} value={`${radiusSteps[radius]} km · ${phonesAt[radius].toLocaleString("en-NG")} phones`}>
            <div data-track className="relative h-1.5 rounded-full bg-stone-2">
              <span className="absolute inset-y-0 left-0 rounded-full bg-blue transition-[width] duration-150" style={{ width: `${fill}%` }} />
              <span
                data-knob
                className="absolute top-1/2 size-5 -translate-1/2 rounded-full bg-white shadow-[0_2px_8px_rgb(13_13_13/0.3)] ring-1 ring-black/10 transition-[left] duration-150"
                style={{ left: `${fill}%` }}
              />
            </div>
          </Row>

          <Row n={3} label="Run for" done={!!days}>
            <div className="flex gap-2">
              {dayChoices.map((d) => (
                <span
                  key={d}
                  data-days={d}
                  className={`rounded-full px-3 py-1.5 text-[0.8rem] font-medium ring-1 transition-colors duration-300 ${
                    days === d ? "bg-ink text-white ring-ink" : "bg-white text-ink ring-black/10"
                  }`}
                >
                  {d} days
                </span>
              ))}
            </div>
          </Row>

          <div className="mt-5 flex items-center justify-between gap-3 border-t pt-4" style={{ borderColor: "var(--line)" }}>
            <p className="text-[0.78rem] leading-tight text-muted-foreground">
              <span className="block font-semibold text-ink">₦30,000</span>
              25,000 views
            </p>
            <span
              data-launch
              className={`inline-flex h-10 items-center gap-2.5 rounded-lg pl-4 pr-1.5 text-[0.88rem] font-medium text-white transition-colors duration-300 ${
                live ? "bg-blue" : "bg-ink"
              }`}
            >
              {live ? "Campaign live" : "Launch campaign"}
              <span className="grid size-7 place-items-center rounded-md bg-yellow text-ink">
                {live ? <Check className="size-3.5" strokeWidth={3} /> : <TriangleArrow />}
              </span>
            </span>
          </div>
        </div>

        {/* Where it lands */}
        <div className="hidden w-[15rem] lg:block">
          <Phone label={`Lock screen showing a ${chosen.ad.brand} notification`}>
            <LockScreen niche={chosen} />
          </Phone>
        </div>
        <div className="w-full max-w-[30rem] lg:hidden">
          <p className="mb-2.5 text-center text-[0.74rem] font-medium text-muted-foreground">
            {live ? `On a phone ${chosen.ad.distance} away, right now` : "Lands on phones near you"}
          </p>
          <div className="ios-scale min-h-[4.5rem] rounded-2xl bg-[linear-gradient(160deg,#c9b48a,#6f6a4c)] p-2.5">
            <NotificationAd niche={chosen} />
          </div>
        </div>
      </div>

      {/* The cursor: a pointer with a name tag, plus the ripple of each click */}
      <div data-cursor aria-hidden="true" className="pointer-events-none invisible absolute left-0 top-0 z-20">
        <span data-tap className="absolute -left-5 -top-5 size-10 rounded-full border-2 border-blue opacity-0" />
        <svg viewBox="0 0 20 22" className="relative size-5 drop-shadow-[0_2px_3px_rgb(0_0_0/0.3)]">
          <path d="M2 1.5 L2 17.5 L6.2 13.6 L9 20 L12 18.7 L9.3 12.5 L15 12.5 Z" fill="#0d0d0d" stroke="white" strokeWidth="1.5" strokeLinejoin="round" />
        </svg>
        <span className="absolute left-4 top-5 rounded-full bg-blue px-2 py-0.5 text-[0.68rem] font-semibold whitespace-nowrap text-white shadow-md">
          You
        </span>
      </div>
    </div>
  );
}

function Row({
  n,
  label,
  done,
  value,
  children,
}: {
  n: number;
  label: string;
  done: boolean;
  value?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mt-5">
      <div className="mb-2.5 flex items-center justify-between gap-3">
        <span className="flex items-center gap-2 text-[0.82rem] font-semibold">
          <span
            className={`grid size-5 place-items-center rounded-full text-[0.66rem] transition-colors duration-300 ${
              done ? "bg-blue text-white" : "bg-stone text-muted-foreground"
            }`}
          >
            {done ? <Check className="size-3" strokeWidth={3} /> : n}
          </span>
          {label}
        </span>
        {value && <span className="text-[0.76rem] text-muted-foreground">{value}</span>}
      </div>
      {children}
    </div>
  );
}
