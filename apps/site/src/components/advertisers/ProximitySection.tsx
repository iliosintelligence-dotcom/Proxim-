"use client";

import { useMemo, useState } from "react";
import type React from "react";
import { Check, MapPin, Radar, Search, SlidersHorizontal } from "lucide-react";
import { cities, radiusSteps } from "@/lib/data";
import { estimateReach } from "@/lib/nigeria";
import { commas } from "@/lib/format";
import { Section } from "@/components/site/Frame";
import { HIcon } from "@/components/site/HIcon";
import { Odometer } from "@/components/motion/Odometer";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Globe } from "./Globe";
import { useCampaign } from "./CampaignContext";

/**
 * Proximity. A dotted globe turned to Nigeria with a city picker laid over
 * it, the way a real targeting panel would sit over a map. Pick a city and
 * the globe turns to it; drag the radius and the phones in range light up
 * with arcs running out to them.
 */
export function ProximitySection() {
  const { city: cityId, radius, set } = useCampaign();
  const [query, setQuery] = useState("");
  const city = cities.find((c) => c.id === cityId)!;
  const reach = estimateReach(city.density, radius);
  const stepIndex = radiusSteps.indexOf(radius);
  const list = useMemo(
    () => cities.filter((c) => `${c.name} ${c.state}`.toLowerCase().includes(query.trim().toLowerCase())),
    [query],
  );

  return (
    <Section id="proximity" index="03" label="Proximity" tone="night" frameClassName="!pb-0" icon={Radar} pip="Drag the radius and watch the globe light up.">
      <div className="grid gap-6 lg:grid-cols-[1.1fr_0.9fr] lg:items-end">
        <h2 data-led className="t-section max-w-[17ch]">
          Only reach people who can <HIcon photo="1687422808191-93810cd07ab0" /> actually walk in.
        </h2>
        <p data-reveal className="t-lead max-w-[30rem] !text-white/55">
          Pick a city, then set the radius. Your ad only shows on phones inside it, so a bakery in Yaba never pays for
          views in Kano.
        </p>
      </div>

      <div className="relative mt-12 lg:mt-16">
        {/* Globe, cropped by the bottom of the section like a horizon. */}
        <div className="relative mx-auto h-[min(92vw,34rem)] overflow-hidden [mask-image:radial-gradient(ellipse_72%_70%_at_50%_48%,black_52%,transparent_100%)] lg:mx-0 lg:h-[41rem] lg:w-[64%]">
          <div className="absolute left-1/2 top-0 w-[min(118vw,44rem)] -translate-x-1/2 lg:w-[50rem]">
            <Globe cityId={cityId} radius={radius} />
          </div>
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-night to-transparent" />
        </div>

        {/* City picker, laid over the globe on desktop. */}
        <div
          data-reveal
          className="relative z-10 -mt-24 w-full rounded-2xl bg-white p-4 text-ink shadow-[0_30px_80px_-20px_rgb(0_0_0/0.6)] ring-1 ring-black/5 sm:p-5 lg:absolute lg:right-0 lg:top-0 lg:mt-0 lg:w-[23rem]"
        >
          <div className="flex items-center justify-between">
            <p className="flex items-center gap-2 text-[0.95rem] font-semibold">
              <MapPin className="size-4" /> Choose a city
            </p>
            <Badge variant="secondary" className="gap-1">
              <span className="size-1.5 rounded-full bg-[#22c55e]" /> 8 live
            </Badge>
          </div>
          <p className="mt-1 text-[0.8rem] text-muted-foreground">Ads only show inside the radius you set.</p>

          <div className="mt-4 flex gap-2">
            <div className="relative flex-1">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search cities"
                aria-label="Search cities"
                className="h-9 pl-8 text-[0.85rem]"
              />
            </div>
            <span className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-input px-3 text-[0.8rem] font-medium">
              <SlidersHorizontal className="size-3.5" /> Filter
            </span>
          </div>

          <div className="mt-3 overflow-hidden rounded-xl border border-border">
            <div className="flex justify-between bg-muted px-3 py-1.5 text-[0.7rem] font-medium text-muted-foreground">
              <span>City</span>
              <span>Phones in 10 km</span>
            </div>
            <ul role="radiogroup" aria-label="Choose a city" className="max-h-[13.5rem] divide-y divide-border overflow-y-auto" data-lenis-prevent>
              {list.map((c) => {
                const on = c.id === cityId;
                return (
                  <li key={c.id}>
                    <button
                      type="button"
                      role="radio"
                      aria-checked={on}
                      onClick={() => set("city", c.id)}
                      className={`flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors ${on ? "bg-blue-soft" : "hover:bg-muted/60"}`}
                    >
                      <span
                        className={`grid size-4 shrink-0 place-items-center rounded-full border ${on ? "border-blue bg-blue text-white" : "border-input"}`}
                      >
                        {on && <Check className="size-2.5" strokeWidth={3.5} />}
                      </span>
                      <span className="min-w-0 flex-1 leading-tight">
                        <span className="block text-[0.85rem] font-medium">{c.name}</span>
                        <span className="block text-[0.72rem] text-muted-foreground">{c.state} State</span>
                      </span>
                      <span className="text-[0.8rem] font-medium tabular-nums text-ink-2">
                        {commas(Math.round(estimateReach(c.density, 10) / 1000) * 1000)}
                      </span>
                    </button>
                  </li>
                );
              })}
              {list.length === 0 && <li className="px-3 py-4 text-center text-[0.8rem] text-muted-foreground">No city matches “{query}”.</li>}
            </ul>
          </div>

          <div className="mt-4">
            <div className="flex items-baseline justify-between">
              <label htmlFor="radius" className="text-[0.8rem] font-medium">
                Radius around {city.name}
              </label>
              <span className="font-display text-[1.3rem] leading-none tracking-[-0.03em]">
                <Odometer value={`${radius}`} /> km
              </span>
            </div>
            <input
              id="radius"
              type="range"
              min={0}
              max={radiusSteps.length - 1}
              step={1}
              value={stepIndex}
              onChange={(e) => set("radius", radiusSteps[Number(e.target.value)])}
              aria-valuetext={`${radius} kilometres`}
              className="range mt-2"
              style={{ "--fill": `${(stepIndex / (radiusSteps.length - 1)) * 100}%` } as React.CSSProperties}
            />
            <div className="flex justify-between text-[0.7rem] text-muted-foreground">
              <span>Your street</span>
              <span>Your whole state</span>
            </div>
          </div>

          <div className="mt-4 flex items-center justify-between rounded-xl bg-muted px-3.5 py-3">
            <div>
              <p className="text-[0.7rem] font-medium text-muted-foreground">Phones in range</p>
              <p className="mt-1 font-display text-[1.35rem] leading-none tracking-[-0.03em]">
                <Odometer value={commas(Math.round(reach / 100) * 100)} />
              </p>
            </div>
            <div className="text-right">
              <p className="text-[0.7rem] font-medium text-muted-foreground">Spent outside it</p>
              <p className="mt-1 font-display text-[1.35rem] leading-none tracking-[-0.03em] text-[#16a34a]">₦0</p>
            </div>
          </div>
          <p className="mt-2 text-[0.68rem] text-faint">Reach figures are sample estimates.</p>
        </div>
      </div>
    </Section>
  );
}
