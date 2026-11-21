"use client";

import { ChartColumn } from "lucide-react";
import { useRef, useState } from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { commas, naira } from "@/lib/format";
import { Section } from "@/components/site/Frame";
import { Odometer } from "@/components/motion/Odometer";
import { HIcon } from "@/components/site/HIcon";

const daily = [42, 55, 51, 63, 70, 66, 81, 77, 88, 84, 92, 97, 90, 100];
const areas = [
  ["Lekki Phase 1", 38],
  ["Ajah", 21],
  ["Victoria Island", 17],
  ["Ikoyi", 12],
] as const;
const apps = [
  ["Daily Word", 46],
  ["Morning Devotion", 31],
  ["Psalms Daily", 23],
] as const;

/**
 * Reporting is shown as a live dashboard, not described. The bars grow as
 * you scroll into it, and while it's on screen the view count keeps
 * ticking, because that's what the real thing does.
 */
export function ReportingSection() {
  const root = useRef<HTMLDivElement>(null);
  const [views, setViews] = useState(31_482);
  const taps = Math.round(views * 0.046);
  const spent = (views / 1000) * 1200;

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.from("[data-bar]", {
        scaleY: 0,
        transformOrigin: "50% 100%",
        duration: 1,
        ease: "expo.out",
        stagger: 0.04,
        scrollTrigger: { trigger: "[data-chart]", start: "top 85%", once: true },
      });
      gsap.from("[data-meter]", {
        scaleX: 0,
        transformOrigin: "0% 50%",
        duration: 1.2,
        ease: "expo.out",
        stagger: 0.08,
        scrollTrigger: { trigger: "[data-meters]", start: "top 90%", once: true },
      });
      const tick = gsap
        .timeline({ repeat: -1, paused: true })
        .call(() => setViews((v) => v + 3 + Math.floor(Math.random() * 11)))
        .to({}, { duration: 1.6 });
      ScrollTrigger.create({
        trigger: root.current,
        start: "top bottom",
        end: "bottom top",
        onToggle: (s) => (s.isActive ? tick.play() : tick.pause()),
      });
    },
    { scope: root, lazy: true },
  );

  return (
    <Section id="reporting" index="03" label="Reporting" tone="night" icon={ChartColumn} pip="Every view counts up live. Watch the numbers.">
      <div className="grid gap-14 lg:grid-cols-[0.8fr_1.2fr] lg:gap-16">
        <div>
          <h2 data-led className="t-section max-w-[14ch]">
            Watch it <HIcon photo="1761370571806-886404629697" /> work, live.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[30rem] !text-white/55">
            See every view and tap as it happens: which apps they came from, which streets they were on, and what each
            tap cost you. No waiting for an end-of-month report.
          </p>
          <ul className="mt-10 space-y-3 text-[0.95rem] text-white/70">
            {["Views, taps and spend, updated every few seconds", "Top apps and top areas for every campaign", "Pause, extend or widen the radius from the same screen"].map((t) => (
              <li key={t} data-reveal className="flex items-start gap-3">
                <span className="mt-2 size-1.5 shrink-0 bg-blue" />
                {t}
              </li>
            ))}
          </ul>
        </div>

        <div ref={root} data-reveal className="min-w-0 rounded-3xl border border-white/10 bg-night-2 p-4 sm:p-6">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-[0.95rem] font-medium">Grace Assembly · Sunday service</p>
              <p className="text-[0.8rem] text-white/45">Faith apps · 10 km around Lekki · Banner</p>
            </div>
            <span className="t-label inline-flex items-center gap-2 rounded-full bg-white/10 px-2.5 py-1 text-white/80">
              <span className="relative flex size-1.5">
                <span className="absolute inset-0 animate-ping rounded-full bg-ad-lime" />
                <span className="relative size-1.5 rounded-full bg-ad-lime" />
              </span>
              Live
            </span>
          </div>

          <div className="mt-6 grid grid-cols-3 gap-px overflow-hidden rounded-2xl bg-white/10">
            {[
              ["Views", <Odometer key="v" value={commas(views)} />],
              ["Taps", <Odometer key="t" value={commas(taps)} />],
              ["Spent", <Odometer key="s" value={naira(spent)} />],
            ].map(([k, v]) => (
              <div key={k as string} className="bg-night-2 p-3 sm:p-4">
                <p className="t-label text-white/40">{k}</p>
                <p className="mt-2 font-display text-[clamp(1.05rem,2.4vw,1.6rem)] leading-none tracking-[-0.03em]">{v}</p>
              </div>
            ))}
          </div>

          <div data-chart className="mt-6 rounded-2xl border border-white/10 p-4">
            <div className="flex justify-between text-[0.75rem] text-white/40">
              <span>Views per day</span>
              <span>Last 14 days</span>
            </div>
            <div className="mt-4 flex h-28 items-end gap-1 sm:gap-1.5" aria-hidden="true">
              {daily.map((d, i) => (
                <span
                  key={i}
                  data-bar
                  className={`flex-1 rounded-t-[3px] ${i === daily.length - 1 ? "bg-blue" : "bg-white/80"}`}
                  style={{ height: `${d}%` }}
                />
              ))}
            </div>
          </div>

          <div data-meters className="mt-4 grid gap-4 sm:grid-cols-2">
            {[
              ["Top areas", areas],
              ["Top apps", apps],
            ].map(([title, list]) => (
              <div key={title as string} className="rounded-2xl border border-white/10 p-4">
                <p className="text-[0.75rem] text-white/40">{title as string}</p>
                <ul className="mt-3 space-y-2.5">
                  {(list as readonly (readonly [string, number])[]).map(([name, pct]) => (
                    <li key={name} className="text-[0.82rem]">
                      <div className="flex justify-between">
                        <span className="text-white/80">{name}</span>
                        <span className="font-mono text-white/45">{pct}%</span>
                      </div>
                      <div className="mt-1 h-1 rounded-full bg-white/10">
                        <div data-meter className="h-full rounded-full bg-white/70" style={{ width: `${pct * 2}%` }} />
                      </div>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <p className="mt-4 text-[0.72rem] text-white/30">Sample dashboard with illustrative numbers.</p>
        </div>
      </div>
    </Section>
  );
}
