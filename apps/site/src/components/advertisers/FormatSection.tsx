"use client";

import { CircleCheck, Smartphone } from "lucide-react";
import { useState } from "react";
import { advertiserCpm, nicheById, type Format } from "@/lib/data";
import { naira } from "@/lib/format";
import { Section } from "@/components/site/Frame";
import { LivePhone } from "@/components/phone/LivePhone";
import { useCampaign } from "./CampaignContext";
import { HIcon } from "@/components/site/HIcon";

const formats: {
  id: Format;
  title: string;
  pitch: string;
  points: string[];
  bestFor: string;
}[] = [
  {
    id: "banner",
    title: "Banner",
    pitch: "Sits inside the app while they scroll. Always visible, never in the way.",
    points: ["Stays on screen for the whole session", "Taps open your page, map or WhatsApp", "Lower cost per view"],
    bestFor: "Steady awareness: menus, services, shops",
  },
  {
    id: "notification",
    title: "Notification",
    pitch: "Lands on their lock screen when they’re near you. Hard to miss, easy to act on.",
    points: ["Arrives even when the app is closed", "Sent only inside your radius", "Highest tap-through"],
    bestFor: "Time-sensitive offers: events, sales, today only",
  },
];

export function FormatSection() {
  const { niche, format, set } = useCampaign();
  const n = nicheById(niche);
  const [replays, setReplays] = useState({ banner: 0, notification: 0 });

  const choose = (f: Format) => {
    set("format", f);
    setReplays((r) => ({ ...r, [f]: r[f] + 1 }));
  };

  return (
    <Section id="formats" index="02" label="Two formats" tone="stone" icon={Smartphone} pip="Banner or notification? Tap one to see it land.">
      <div className="max-w-[44rem]">
        <h2 data-led className="t-section">
          Two ways to be <HIcon photo="1687422808248-f807f4ea2a2e" /> seen. Both hard to ignore.
        </h2>
        <p data-reveal className="t-lead mt-6">
          Pick the format that fits the message. Tap a card to see your {n.label.toLowerCase()} ad land the way your
          customers will see it.
        </p>
      </div>

      <div role="radiogroup" aria-label="Choose an ad format" className="mt-14 grid gap-5 lg:grid-cols-2">
        {formats.map((f) => {
          const on = format === f.id;
          return (
            <button
              key={f.id}
              type="button"
              role="radio"
              aria-checked={on}
              data-reveal
              onClick={() => choose(f.id)}
              className={`group relative flex flex-col overflow-hidden rounded-3xl border bg-white text-left transition-[border-color,box-shadow] duration-300 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-blue ${
                on
                  ? "border-ink shadow-[0_24px_50px_-28px_rgb(28_25_23/0.45)]"
                  : "border-[var(--line)] hover:border-[var(--line-strong)]"
              }`}
            >
              <div className="flex items-start justify-between gap-4 p-6 pb-0 sm:p-8 sm:pb-0">
                <div>
                  <h3 className="t-card">{f.title}</h3>
                  <p className="mt-2 max-w-[26rem] text-[0.98rem] leading-relaxed text-muted-foreground">{f.pitch}</p>
                </div>
                <span
                  aria-hidden="true"
                  className={`mt-1 grid size-6 shrink-0 place-items-center rounded-full border transition-colors ${on ? "border-ink bg-ink" : "border-[var(--line-strong)]"}`}
                >
                  <span className={`size-2 bg-white transition-transform duration-300 ${on ? "scale-100" : "scale-0"}`} />
                </span>
              </div>

              <div className="relative mt-8 flex h-[19rem] justify-center overflow-hidden sm:h-[22rem]">
                <div className="absolute inset-x-6 bottom-0 top-0 rounded-t-[1.5rem] bg-stone sm:inset-x-8" />
                <LivePhone
                  niche={n}
                  format={f.id}
                  screen={f.id === "notification" ? "lock" : "app"}
                  replayKey={replays[f.id]}
                  // Banner card shows the bottom of the phone, where banners
                  // live. Notification card shows the top of the lock screen.
                  className={`absolute left-1/2 w-[13.5rem] -translate-x-1/2 transition-transform duration-700 ease-[var(--ease-out-quint)] sm:w-[15rem] ${
                    f.id === "banner" ? "-bottom-4 group-hover:-translate-y-2" : "top-6 group-hover:-translate-y-2"
                  }`}
                />
              </div>

              <div className="grid gap-4 border-t p-6 sm:grid-cols-[1fr_auto] sm:items-end sm:p-8" style={{ borderColor: "var(--line)" }}>
                <ul className="space-y-1.5 text-[0.92rem] text-ink-2">
                  {f.points.map((p) => (
                    <li key={p} className="flex items-center gap-2.5">
                      <CircleCheck className="size-4 shrink-0 text-blue" strokeWidth={2.2} />
                      {p}
                    </li>
                  ))}
                </ul>
                <div className="sm:text-right">
                  <p className="t-label text-faint">From</p>
                  <p className="font-display text-[1.35rem] leading-tight tracking-[-0.03em]">
                    {naira(advertiserCpm[f.id])}
                    <span className="font-sans text-[0.8rem] tracking-normal text-muted-foreground"> / 1,000 views</span>
                  </p>
                </div>
                <p className="text-[0.82rem] text-muted-foreground sm:col-span-2">Best for: {f.bestFor}</p>
              </div>
            </button>
          );
        })}
      </div>
    </Section>
  );
}
