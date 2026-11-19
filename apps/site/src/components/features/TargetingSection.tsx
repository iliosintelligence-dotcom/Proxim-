"use client";

import { CalendarDays, Crosshair, MapPin } from "lucide-react";
import { useRef } from "react";
import type React from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { niches, tintBg } from "@/lib/data";
import { Section } from "@/components/site/Frame";
import { NicheIcon } from "@/components/site/NicheIcon";
import { AppIcon, type IconTone } from "@/components/site/AppIcon";
import type { LucideIcon } from "lucide-react";
import { HIcon } from "@/components/site/HIcon";

/**
 * Three filters, three small machines. Each visual runs on a loop while it
 * is on screen, so the card shows the idea without needing a caption.
 */
export function TargetingSection() {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const loops: gsap.core.Timeline[] = [];

      // Niche: one app tile at a time lights up in its colour, the rest stay grey.
      const tiles = gsap.utils.toArray<HTMLElement>("[data-tile]");
      const nicheTl = gsap.timeline({ repeat: -1, paused: true });
      tiles.forEach((t) => {
        nicheTl
          .to(t.querySelector("[data-tint]"), { opacity: 1, duration: 0.25, ease: "steps(2)" })
          .to(t, { y: -4, duration: 0.5, ease: "expo.out" }, "<")
          .to(t.querySelector("[data-tint]"), { opacity: 0, duration: 0.25, ease: "steps(2)" }, "+=0.9")
          .to(t, { y: 0, duration: 0.5, ease: "expo.out" }, "<");
      });
      loops.push(nicheTl);

      // Proximity: sonar from the pin.
      const sonarTl = gsap.timeline({ repeat: -1, paused: true });
      sonarTl.fromTo(
        "[data-ring]",
        { scale: 0.1, opacity: 0.9 },
        { scale: 1, opacity: 0, duration: 2.6, ease: "power2.out", stagger: 0.85, transformOrigin: "50% 50%" },
      );
      loops.push(sonarTl);

      // Pacing: views spread evenly across the days.
      const days = gsap.utils.toArray<HTMLElement>("[data-day]");
      const paceTl = gsap.timeline({ repeat: -1, repeatDelay: 1.2, paused: true });
      paceTl
        .set(days, { scaleY: 0 })
        .to(days, { scaleY: 1, duration: 0.5, ease: "expo.out", stagger: 0.12, transformOrigin: "50% 100%" });
      loops.push(paceTl);

      ScrollTrigger.create({
        trigger: root.current,
        start: "top 85%",
        end: "bottom 15%",
        onToggle: (s) => loops.forEach((l) => (s.isActive ? l.play() : l.pause())),
      });
    },
    { scope: root },
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

      <div ref={root} className="mt-14 grid gap-4 lg:grid-cols-3">
        <Card icon={Crosshair} tone="blue" title="Niche matching" body="Pick what you sell. Proxim places you only in apps whose users already care about it: scripture readers, runners, home cooks, students.">
          <div className="grid grid-cols-3 gap-2 p-6">
            {niches.map((n) => (
              <div key={n.id} data-tile className="relative flex flex-col items-center gap-1.5 overflow-hidden rounded-xl border border-[var(--line)] bg-white px-2 py-2.5 text-center">
                <span data-tint className={`absolute inset-0 opacity-0 ${tintBg[n.ad.tint]}`} />
                <NicheIcon niche={n} size="sm" className="relative" />
                <span className="relative block text-[0.74rem] font-medium">{n.label}</span>
              </div>
            ))}
          </div>
        </Card>

        <Card icon={MapPin} tone="orange" title="Proximity" body="Draw a radius from 2 km to 200 km around your business, or several branches at once. Phones outside it never see your ad, and you never pay for them.">
          <div className="relative grid h-[9.5rem] place-items-center overflow-hidden">
            <div
              className="absolute inset-0 opacity-60"
              style={{
                backgroundImage: "radial-gradient(circle, rgb(28 25 23 / 0.16) 1.2px, transparent 1.6px)",
                backgroundSize: "12px 12px",
              }}
            />
            {[0, 1, 2].map((i) => (
              <span key={i} data-ring className="absolute size-44 rounded-full border-2 border-blue/50 opacity-0" />
            ))}
            <span className="relative size-3.5 bg-blue ring-4 ring-white" />
          </div>
        </Card>

        <div id="scheduling" className="scroll-mt-24">
          <Card icon={CalendarDays} tone="purple" title="Duration and views" body="Choose the dates and the number of views. Proxim spreads them evenly across the run, so day one doesn’t spend the budget meant for day ten.">
            <div className="flex h-[9.5rem] items-end gap-1.5 px-6 pb-6 pt-4">
              {Array.from({ length: 10 }, (_, i) => (
                <span key={i} className="flex flex-1 flex-col items-center gap-1.5">
                  <span data-day className="block h-16 w-full rounded-[3px] bg-ink" />
                  <span className="text-[0.62rem] text-faint">D{i + 1}</span>
                </span>
              ))}
            </div>
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
      <div className="border-b bg-stone" style={{ borderColor: "var(--line)" }}>
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
