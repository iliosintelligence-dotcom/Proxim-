"use client";

import { CalendarDays, Crosshair, MapPin } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type React from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { niches, tintBg } from "@/lib/data";
import { Section } from "@/components/site/Frame";
import { NicheIcon } from "@/components/site/NicheIcon";
import { AppIcon, type IconTone } from "@/components/site/AppIcon";
import type { LucideIcon } from "lucide-react";
import { HIcon } from "@/components/site/HIcon";
import { PipMark } from "@/components/mascot/PipShape";

/**
 * Three filters, three small machines. Each visual plays on its own while
 * it is on screen and can be taken over by hand: tap a radius, flip the
 * pacing. All three visuals share one height so the cards' text lines up.
 */
export function TargetingSection() {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;

      // Niche: one app tile at a time lights up in its colour, the rest stay grey.
      const tiles = gsap.utils.toArray<HTMLElement>("[data-tile]");
      const nicheTl = gsap.timeline({ repeat: -1, paused: true });
      tiles.forEach((t) => {
        nicheTl
          .to(t.querySelector("[data-tint]"), { opacity: 1, duration: 0.3, ease: "power2.out" })
          .to(t, { y: -4, duration: 0.5, ease: "expo.out" }, "<")
          .to(t.querySelector("[data-tint]"), { opacity: 0, duration: 0.3, ease: "power2.in" }, "+=0.9")
          .to(t, { y: 0, duration: 0.5, ease: "expo.out" }, "<");
      });

      // Proximity: sonar rings from Pip.
      const sonarTl = gsap.timeline({ repeat: -1, paused: true });
      sonarTl.fromTo(
        "[data-ring]",
        { scale: 0.15, opacity: 0.8 },
        { scale: 1, opacity: 0, duration: 2.6, ease: "power2.out", stagger: 0.85 },
      );

      ScrollTrigger.create({
        trigger: root.current,
        start: "top 85%",
        end: "bottom 15%",
        onToggle: (s) => [nicheTl, sonarTl].forEach((l) => (s.isActive ? l.play() : l.pause())),
      });
    },
    { scope: root, lazy: true },
  );

  return (
    <Section id="targeting" index="01" label="Targeting" icon={Crosshair} pip="Niche, place and time. Get all three right.">
      <div className="max-w-[46rem]">
        <h2 data-led className="t-section">
          Three settings. One <HIcon photo="1702236240794-58dc4c6895e5" /> right person.
        </h2>
        <p data-reveal className="t-lead mt-6">
          Every Proxim campaign is the overlap of three choices: the kind of app, the area around you, and how long
          and how often it runs. Get the overlap right and almost nobody who sees your ad is the wrong person.
        </p>
      </div>

      <div ref={root} className="mt-14 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <Card
          icon={Crosshair}
          tone="blue"
          title="Niche matching"
          body="Pick what you sell. Proxim places you only in apps whose users already care about it: scripture readers, runners, home cooks, students."
        >
          <div className="grid h-full grid-cols-3 content-center gap-2 px-6">
            {niches.map((n) => (
              <div
                key={n.id}
                data-tile
                className="relative flex flex-col items-center gap-1.5 overflow-hidden rounded-2xl border border-[var(--line)] bg-white px-2 py-3 text-center"
              >
                <span data-tint className={`absolute inset-0 opacity-0 ${tintBg[n.ad.tint]}`} />
                <NicheIcon niche={n} size="sm" className="relative" />
                <span className="relative block text-[0.74rem] font-medium">{n.label}</span>
              </div>
            ))}
          </div>
        </Card>

        <Card
          icon={MapPin}
          tone="orange"
          title="Proximity"
          body="Draw a radius from 2 km to 200 km around your business, or several branches at once. Phones outside it never see your ad, and you never pay for them."
        >
          <RadiusMap />
        </Card>

        <div id="scheduling" className="scroll-mt-24 md:col-span-2 lg:col-span-1">
          <Card
            icon={CalendarDays}
            tone="purple"
            title="Duration and views"
            body="Choose the dates and the number of views. Proxim spreads them evenly across the run, so day one doesn’t spend the budget meant for day ten."
          >
            <Pacing />
          </Card>
        </div>
      </div>
    </Section>
  );
}

function Card({
  icon,
  tone,
  title,
  body,
  children,
}: {
  icon: LucideIcon;
  tone: IconTone;
  title: string;
  body: string;
  children: React.ReactNode;
}) {
  return (
    <article data-reveal className="flex h-full flex-col overflow-hidden rounded-3xl border border-[var(--line)] bg-stone/60">
      {/* Every visual is the same height, so icon, title and text line up across the row. */}
      <div className="relative h-[15.5rem] shrink-0 border-b bg-stone" style={{ borderColor: "var(--line)" }}>
        {children}
      </div>
      <div className="p-6">
        <AppIcon icon={icon} tone={tone} />
        <h3 className="t-card mt-4">{title}</h3>
        <p className="mt-2 text-[0.94rem] leading-relaxed text-muted-foreground">{body}</p>
      </div>
    </article>
  );
}

/** True while the element is on screen; drives the self-playing demos. */
function useOnScreen<T extends Element>() {
  const ref = useRef<T>(null);
  const [on, setOn] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(([e]) => setOn(e.isIntersecting), { threshold: 0.4 });
    io.observe(el);
    return () => io.disconnect();
  }, []);
  return [ref, on] as const;
}

/** Plays `step` every `ms` while on screen, until the visitor takes over. */
function useAutoplay(on: boolean, touched: boolean, ms: number, step: () => void) {
  const saved = useRef(step);
  saved.current = step;
  useEffect(() => {
    if (!on || touched || prefersReducedMotion()) return;
    const id = window.setInterval(() => saved.current(), ms);
    return () => window.clearInterval(id);
  }, [on, touched, ms]);
}

/* ------------------------------------------------------------------ */
/* Proximity: a little map with Pip in the middle.                     */
/* ------------------------------------------------------------------ */

const radii = [
  { km: 2, px: 38 },
  { km: 5, px: 58 },
  { km: 10, px: 80 },
  { km: 25, px: 104 },
];

// People around the shop, as offsets from the centre in px.
const people = [
  [-22, -18], [26, 12], [-8, 30], [34, -26], [-44, 8], [12, -48], [52, 30], [-58, -36],
  [-30, 56], [70, -6], [-78, 18], [44, -62], [-12, -74], [88, 40], [-96, -14], [20, 78],
  [-66, 66], [104, -44], [-110, 52], [118, 8], [-40, -88], [76, 82],
] as const;

function RadiusMap() {
  const [ref, on] = useOnScreen<HTMLDivElement>();
  const [i, setI] = useState(1);
  const [touched, setTouched] = useState(false);
  useAutoplay(on, touched, 2200, () => setI((n) => (n + 1) % radii.length));
  const r = radii[i];
  const inRange = people.filter(([x, y]) => Math.hypot(x, y) <= r.px).length;

  return (
    <div ref={ref} className="absolute inset-0 overflow-hidden">
      {/* Streets */}
      <svg aria-hidden="true" className="absolute inset-0 size-full" preserveAspectRatio="xMidYMid slice" viewBox="0 0 320 248">
        <g fill="none" stroke="white" strokeLinecap="round">
          <path d="M-10 70 C 80 60, 160 96, 330 74" strokeWidth="9" />
          <path d="M-10 182 C 90 196, 210 160, 330 176" strokeWidth="9" />
          <path d="M92 -10 C 104 80, 84 170, 110 260" strokeWidth="7" />
          <path d="M238 -10 C 222 90, 246 170, 226 260" strokeWidth="7" />
          <path d="M-10 128 L 330 120" strokeWidth="4" opacity="0.8" />
          <path d="M164 -10 L 172 260" strokeWidth="4" opacity="0.8" />
        </g>
      </svg>

      <div className="absolute left-1/2 top-[44%]">
        {/* The radius */}
        <span
          className="absolute rounded-full border-2 border-blue bg-blue/10 transition-[width,height,left,top] duration-700 ease-[var(--ease-out-quint)]"
          style={{ width: r.px * 2, height: r.px * 2, left: -r.px, top: -r.px }}
        />
        {[0, 1, 2].map((k) => (
          <span
            key={k}
            data-ring
            className="absolute rounded-full border border-blue/60 opacity-0"
            style={{ width: r.px * 2, height: r.px * 2, left: -r.px, top: -r.px }}
          />
        ))}
        {/* People: blue when they're in range */}
        {people.map(([x, y], k) => {
          const inside = Math.hypot(x, y) <= r.px;
          return (
            <span
              key={k}
              className={`absolute size-2 -translate-1/2 rounded-full transition-[background-color,box-shadow,scale] duration-500 ${
                inside ? "scale-125 bg-blue shadow-[0_0_0_3px_rgb(0_84_249/0.18)]" : "bg-ink/20"
              }`}
              style={{ left: x, top: y, transitionDelay: `${Math.hypot(x, y) * 2}ms` }}
            />
          );
        })}
        {/* The business */}
        <span className="absolute grid size-9 -translate-1/2 place-items-center rounded-full bg-white shadow-[0_6px_16px_-6px_rgb(13_13_13/0.45)]">
          <PipMark className="h-5 w-auto" cheeks={false} />
        </span>
      </div>

      {/* Readout + control */}
      <div className="absolute inset-x-4 bottom-3 rounded-2xl bg-white/90 px-3.5 pb-1.5 pt-2.5 shadow-[0_8px_20px_-12px_rgb(13_13_13/0.35)] ring-1 ring-black/5 backdrop-blur">
        <div className="flex items-baseline justify-between text-[0.78rem]">
          <span className="font-semibold">{r.km} km around you</span>
          <span className="text-muted-foreground">
            <span className="font-semibold text-blue">{(inRange * 380).toLocaleString("en-NG")}</span> phones in range
          </span>
        </div>
        <input
          type="range"
          min={0}
          max={radii.length - 1}
          step={1}
          value={i}
          onChange={(e) => {
            setTouched(true);
            setI(Number(e.target.value));
          }}
          aria-label="Radius"
          aria-valuetext={`${r.km} kilometres`}
          className="range"
          style={{ "--fill": `${(i / (radii.length - 1)) * 100}%`, "--track-fill": "var(--color-blue)" } as React.CSSProperties}
        />
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/* Duration and views: the same budget, paced or not.                  */
/* ------------------------------------------------------------------ */

const DAYS = 10;
const even = Array.from({ length: DAYS }, () => 0.55);
// Without pacing, the busy first days eat the budget.
const rushed = [1, 0.8, 0.55, 0.3, 0.12, 0, 0, 0, 0, 0];

function Pacing() {
  const [ref, on] = useOnScreen<HTMLDivElement>();
  const [paced, setPaced] = useState(true);
  const [touched, setTouched] = useState(false);
  useAutoplay(on, touched, 2600, () => setPaced((p) => !p));
  const heights = paced ? even : rushed;
  const choose = (v: boolean) => {
    setTouched(true);
    setPaced(v);
  };

  return (
    <div ref={ref} className="flex h-full flex-col px-6 pb-4 pt-5">
      <div role="radiogroup" aria-label="Pacing" className="flex w-fit gap-1 rounded-full bg-white p-1 text-[0.76rem] font-medium shadow-[0_4px_12px_-8px_rgb(13_13_13/0.3)] ring-1 ring-black/5">
        {[
          { v: true, label: "Even pacing" },
          { v: false, label: "No pacing" },
        ].map((o) => (
          <button
            key={o.label}
            type="button"
            role="radio"
            aria-checked={paced === o.v}
            onClick={() => choose(o.v)}
            className={`rounded-full px-3 py-1.5 transition-colors duration-300 ${
              paced === o.v ? (o.v ? "bg-blue text-white" : "bg-ink text-white") : "text-muted-foreground hover:text-ink"
            }`}
          >
            {o.label}
          </button>
        ))}
      </div>

      <div className="mt-auto flex h-[7.5rem] items-end gap-1.5" aria-hidden="true">
        {heights.map((h, d) => (
          <span key={d} className="flex h-full flex-1 flex-col items-center justify-end gap-1.5">
            <span className="relative flex w-full flex-1 items-end justify-center">
              {/* Ghost of an even day, so empty days read as lost views */}
              <span className="absolute bottom-0 h-[55%] w-full rounded-full border border-dashed border-ink/15" />
              <span
                className={`relative w-full rounded-full transition-[height,background-color] duration-700 ease-[var(--ease-out-quint)] ${
                  paced ? "bg-gradient-to-t from-blue to-[#4d8bff]" : "bg-gradient-to-t from-ink to-[#4a4a4a]"
                }`}
                style={{ height: `${h * 100}%`, transitionDelay: `${d * 35}ms` }}
              />
            </span>
            <span className="text-[0.6rem] text-faint">D{d + 1}</span>
          </span>
        ))}
      </div>

      <p className="mt-3 text-[0.78rem] font-medium" aria-live="polite">
        {paced ? (
          <>
            <span className="text-blue">2,500 views a day</span>, all ten days
          </>
        ) : (
          <>Budget gone by day 5. Days 6 to 10 get nothing.</>
        )}
      </p>
    </div>
  );
}
