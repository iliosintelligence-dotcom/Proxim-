"use client";

import { ShieldCheck } from "lucide-react";
import { useState } from "react";
import type React from "react";
import { niches, nicheById, type NicheId } from "@/lib/data";
import { Section } from "@/components/site/Frame";
import { LivePhone } from "@/components/phone/LivePhone";
import { Phone } from "@/components/phone/Phone";
import { Switch } from "@/components/site/Switch";
import { NicheIcon } from "@/components/site/NicheIcon";
import { HIcon } from "@/components/site/HIcon";

const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/**
 * Control is the app owner's biggest fear about ads, so it gets a section
 * you can actually operate. Turn categories off and the ad in the phone
 * changes to one you still allow. Turn them all off and nothing shows.
 */
export function ControlSection() {
  const [allowed, setAllowed] = useState<NicheId[]>(["faith", "food", "education", "fitness"]);
  const [perWeek, setPerWeek] = useState(2);
  const [review, setReview] = useState(true);
  const current = allowed.length ? nicheById(allowed[0]) : null;

  const toggle = (id: NicheId) =>
    setAllowed((a) => (a.includes(id) ? a.filter((x) => x !== id) : [...a, id]));

  return (
    <Section id="control" index="02" label="Your rules" tone="stone" icon={ShieldCheck} pip="You decide what shows. Try turning a category off.">
      <div className="grid items-center gap-14 lg:grid-cols-[1.05fr_1fr] lg:gap-20">
        <div>
          <h2 data-led className="t-section max-w-[18ch]">
            Your app. Your <HIcon photo="1668875438804-5f472c3bf565" /> rules. Always.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[34rem]">
            Choose which kinds of business can appear, how often notifications go out, and whether you approve each
            advertiser first. Change it any time, and it applies straight away.
          </p>

          <div data-reveal className="mt-10">
            <p className="t-label text-faint">Allowed categories</p>
            <div className="mt-3 flex flex-wrap gap-2">
              {niches.map((n) => {
                const on = allowed.includes(n.id);
                return (
                  <button
                    key={n.id}
                    type="button"
                    aria-pressed={on}
                    onClick={() => toggle(n.id)}
                    className={`inline-flex items-center gap-2 rounded-full border py-1 pl-1 pr-3.5 text-[0.88rem] transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${
                      on
                        ? "border-ink bg-white text-ink"
                        : "border-[var(--line-strong)] text-muted-foreground line-through decoration-ink/30 hover:text-ink"
                    }`}
                  >
                    <NicheIcon niche={n} size="xs" muted={!on} />
                    {n.label}
                  </button>
                );
              })}
            </div>
          </div>

          <div data-reveal className="mt-9">
            <div className="flex items-baseline justify-between">
              <label htmlFor="perweek" className="t-label text-faint">
                Notification ads per user, per week
              </label>
              <span className="font-display text-[1.6rem] leading-none tracking-[-0.03em]">{perWeek === 0 ? "Off" : perWeek}</span>
            </div>
            <input
              id="perweek"
              type="range"
              min={0}
              max={3}
              value={perWeek}
              onChange={(e) => setPerWeek(Number(e.target.value))}
              className="range mt-3"
              style={{ "--fill": `${(perWeek / 3) * 100}%` } as React.CSSProperties}
            />
            <div className="mt-4 grid grid-cols-7 gap-1.5" aria-hidden="true">
              {DAYS.map((d, i) => {
                // Spread notifications evenly through the week.
                const lit = perWeek > 0 && [[], [3], [1, 4], [0, 3, 5]][perWeek].includes(i);
                return (
                  <div key={d} className="text-center">
                    <div
                      className={`grid aspect-square place-items-center rounded-lg border transition-colors duration-300 ${lit ? "border-ink bg-ink" : "border-[var(--line)] bg-white"}`}
                    >
                      <span className={`size-1.5 transition-transform duration-300 ${lit ? "scale-100 bg-white" : "scale-0"}`} />
                    </div>
                    <span className="mt-1 block text-[0.7rem] text-faint">{d}</span>
                  </div>
                );
              })}
            </div>
          </div>

          <div data-reveal className="mt-9 flex items-center justify-between gap-6 rounded-2xl border border-[var(--line)] bg-white p-4">
            <span>
              <span className="block font-medium">Approve every advertiser</span>
              <span className="block text-[0.85rem] text-muted-foreground">New brands wait for your OK before going live.</span>
            </span>
            <Switch on={review} onChange={setReview} label="Approve every advertiser" />
          </div>
        </div>

        <div className="relative flex flex-col items-center">
          {current ? (
            <LivePhone niche={current} host="faith" format="banner" className="w-[min(17rem,72vw)]" />
          ) : (
            <div className="w-[min(17rem,72vw)]">
              <Phone label="Daily Word app with no ads">
                <EmptyState />
              </Phone>
            </div>
          )}
          <p data-reveal className="mt-6 max-w-[20rem] text-center text-[0.88rem] text-muted-foreground">
            {current
              ? `Showing ${current.ad.brand} because ${current.label.toLowerCase()} ads are allowed.`
              : "Every category is off, so no ads will show in your app."}
          </p>
        </div>
      </div>
    </Section>
  );
}

function EmptyState() {
  return (
    <div className="absolute inset-0 grid place-items-center bg-white px-[2em] text-center">
      <div>
        <p className="font-display text-[1.5em] leading-tight tracking-[-0.03em]">No ads</p>
        <p className="mt-[0.4em] text-[0.85em] text-ink/50">Turn on a category to start earning again.</p>
      </div>
    </div>
  );
}
