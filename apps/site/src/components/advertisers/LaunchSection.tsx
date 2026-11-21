"use client";

import { Calculator, Wallet } from "lucide-react";
import { useEffect, useRef } from "react";
import type React from "react";
import { advertiserCpm, cities, impressionSteps, nicheById, niches, type Format } from "@/lib/data";
import { commas, compact, naira } from "@/lib/format";
import { estimateReach } from "@/lib/nigeria";
import { Photo } from "@/components/phone/Photo";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Section } from "@/components/site/Frame";
import { NicheIcon } from "@/components/site/NicheIcon";
import { Button } from "@/components/site/Button";
import { Odometer } from "@/components/motion/Odometer";
import { useCampaign } from "./CampaignContext";

const MAX_DAYS = 30;

/**
 * Everything chosen further up the page arrives here as one ticket, priced
 * live. The visitor has effectively built their first campaign before
 * they've signed up, which is the strongest reason to sign up.
 */
export function LaunchSection({
  id = "launch",
  index = "04",
  label = "Pricing",
  withPickers = false,
  title = "Pay for views. Not for hope.",
}: {
  id?: string;
  index?: string;
  label?: string;
  /** On the pricing page the earlier sections aren't there, so let people pick here. */
  withPickers?: boolean;
  title?: React.ReactNode;
} = {}) {
  const c = useCampaign();
  const niche = nicheById(c.niche);
  const city = cities.find((x) => x.id === c.city)!;
  const total = (c.impressions / 1000) * advertiserCpm[c.format];
  const perDay = c.impressions / c.days;
  const impIndex = impressionSteps.indexOf(c.impressions);
  const days = useRef<HTMLDivElement>(null);

  // Pip cheers when the price changes.
  const firstPrice = useRef(true);
  useEffect(() => {
    if (firstPrice.current) {
      firstPrice.current = false;
      return;
    }
    window.dispatchEvent(new Event("pip:happy"));
  }, [total]);
  const prevDays = useRef(c.days);

  // New days pop in one after another; removed days just switch off.
  useGSAP(
    () => {
      const from = prevDays.current;
      prevDays.current = c.days;
      if (prefersReducedMotion() || c.days <= from) return;
      const added = Array.from(days.current!.children).slice(from, c.days);
      gsap.fromTo(added, { scale: 0.2 }, { scale: 1, duration: 0.5, ease: "back.out(3)", stagger: 0.02 });
    },
    { dependencies: [c.days], scope: days },
  );

  return (
    <Section id={id} index={index} label={label} icon={withPickers ? Calculator : Wallet} pip="That’s the full price. No surprises, no contract.">
      <div className="grid gap-14 lg:grid-cols-[1fr_minmax(0,27rem)] lg:gap-20">
        <div>
          <h2 data-led className="t-section max-w-[16ch]">
            {title}
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[33rem]">
            Choose how long it runs and how many people should see it. You see the full price before you pay, and
            you’re only charged for views that are actually delivered.
          </p>

          <div data-reveal className="mt-12 space-y-10">
            {withPickers && <Pickers />}
            <div>
              <div className="flex items-baseline justify-between">
                <label htmlFor="days" className="t-label text-faint">
                  How long it runs
                </label>
                <span className="font-display text-[1.6rem] leading-none tracking-[-0.03em]">
                  <Odometer value={`${c.days}`} /> {c.days === 1 ? "day" : "days"}
                </span>
              </div>
              <input
                id="days"
                type="range"
                min={1}
                max={MAX_DAYS}
                value={c.days}
                onChange={(e) => c.set("days", Number(e.target.value))}
                className="range mt-3"
                style={{ "--fill": `${((c.days - 1) / (MAX_DAYS - 1)) * 100}%` } as React.CSSProperties}
              />
            </div>

            <div>
              <div className="flex items-baseline justify-between">
                <label htmlFor="views" className="t-label text-faint">
                  How many views
                </label>
                <span className="font-display text-[1.6rem] leading-none tracking-[-0.03em]">
                  <Odometer value={commas(c.impressions)} />
                </span>
              </div>
              <input
                id="views"
                type="range"
                min={0}
                max={impressionSteps.length - 1}
                value={impIndex}
                onChange={(e) => c.set("impressions", impressionSteps[Number(e.target.value)])}
                aria-valuetext={`${commas(c.impressions)} views`}
                className="range mt-3"
                style={{ "--fill": `${(impIndex / (impressionSteps.length - 1)) * 100}%` } as React.CSSProperties}
              />
              <div className="mt-1 flex justify-between text-[0.75rem] text-faint">
                <span>{compact(impressionSteps[0])}</span>
                <span>{compact(impressionSteps[impressionSteps.length - 1])}</span>
              </div>
            </div>
          </div>
        </div>

        <div data-reveal className="lg:sticky lg:top-24 lg:self-start">
          <div className="overflow-hidden rounded-3xl border border-[var(--line)] bg-white shadow-[0_30px_60px_-30px_rgb(28_25_23/0.35)]">
            <div className="flex items-center justify-between px-6 pb-4 pt-6">
              <p className="font-display text-[1.2rem] tracking-[-0.02em]">Your campaign</p>
              <span className="t-label rounded-full bg-stone px-2.5 py-1 text-muted-foreground">Draft</span>
            </div>

            <dl className="divide-y px-6 text-[0.92rem]" style={{ borderColor: "var(--line)" }}>
              <Row label="Niche">
                <span className="inline-flex items-center gap-2">
                  <NicheIcon niche={niche} size="xs" />
                  {niche.label} apps
                </span>
              </Row>
              <Row label="Format">{c.format === "banner" ? "In-app banner" : "Lock-screen notification"}</Row>
              <Row label="Where">
                {c.radius} km around {city.name}
              </Row>
              <Row label="Views">
                {commas(c.impressions)} <span className="text-muted-foreground">· about {commas(perDay)} a day</span>
              </Row>
            </dl>

            <div className="px-6 pb-5 pt-4">
              <p className="t-label text-faint">Runs for {c.days} {c.days === 1 ? "day" : "days"}</p>
              <div ref={days} className="mt-2.5 grid grid-cols-[repeat(15,1fr)] gap-x-1 gap-y-1.5" aria-hidden="true">
                {Array.from({ length: MAX_DAYS }, (_, i) => (
                  <span
                    key={i}
                    className={`h-4 rounded-full transition-colors duration-300 ${i < c.days ? "bg-blue" : "bg-stone-2"}`}
                  />
                ))}
              </div>
            </div>

            <div className="perforation relative h-4">
              <span className="absolute -left-2 top-1/2 size-4 -translate-y-1/2 rounded-full bg-paper ring-1 ring-[var(--line)]" />
              <span className="absolute -right-2 top-1/2 size-4 -translate-y-1/2 rounded-full bg-paper ring-1 ring-[var(--line)]" />
            </div>

            <div className="bg-stone/50 px-6 pb-6 pt-5">
              <div className="flex items-end justify-between gap-4">
                <div>
                  <p className="t-label text-faint">Total</p>
                  <p className="mt-1.5 font-display text-[2.6rem] leading-none tracking-[-0.04em]">
                    <Odometer value={naira(total)} />
                  </p>
                </div>
                <p className="pb-1 text-right text-[0.78rem] leading-snug text-muted-foreground">
                  {naira(advertiserCpm[c.format])}
                  <br />
                  per 1,000 views
                </p>
              </div>
              <Button href="#" arrow className="mt-5 w-full">
                Launch this campaign
              </Button>
              <p className="mt-3 text-center text-[0.75rem] text-faint">Sample rates shown. No card needed to start.</p>
            </div>
          </div>
        </div>
      </div>
      <CampaignOutcomes total={total} />
    </Section>
  );
}

const pillStyles = [
  "bg-[linear-gradient(90deg,#2f6bff,#22c1ee)]",
  "bg-[linear-gradient(90deg,#ff8a1f,#ffb21f)]",
  "bg-[linear-gradient(90deg,#ff3d9a,#b33dff)]",
  "bg-[linear-gradient(90deg,#16b07a,#19d3c5)]",
  "bg-[linear-gradient(90deg,#9ad81f,#e0f03a)] !text-ink",
  "bg-[linear-gradient(90deg,#5b5bff,#9b6bff)]",
];
const pillOffsets = ["lg:translate-x-10", "lg:translate-x-0", "lg:-translate-x-6", "lg:translate-x-0", "lg:translate-x-10", "lg:translate-x-20"];

/**
 * What the campaign above would deliver, as a column of result pills
 * (after the "Achieve your goals" reference). Every number is computed
 * from the live ticket, so dragging a slider rolls the figures here too:
 * the price stops being a cost and becomes an outcome.
 */
function CampaignOutcomes({ total }: { total: number }) {
  const c = useCampaign();
  const city = cities.find((x) => x.id === c.city)!;
  const taps = c.impressions * 0.046;
  const visits = taps * 0.18;
  const reach = estimateReach(city.density, c.radius);
  const root = useRef<HTMLDivElement>(null);

  const pills: { photo: string; value: string; label: string }[] = [
    { photo: "1687422808565-929533931584", value: commas(c.impressions), label: "Views" },
    { photo: "1687422809654-579d81c29d32", value: compact(Math.round(reach / 1000) * 1000), label: "Phones in range" },
    { photo: "1687422808311-a776f467a468", value: commas(taps), label: "Estimated taps" },
    { photo: "1687422808191-93810cd07ab0", value: commas(visits), label: "Estimated store visits" },
    { photo: "1709837167686-a2e33aad1bf0", value: `₦${(advertiserCpm[c.format] / 1000).toFixed(2)}`, label: "Per view" },
    { photo: "1698827623494-c4e1196ba0ca", value: `${c.days}`, label: c.days === 1 ? "Day on air" : "Days on air" },
  ];

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const items = gsap.utils.toArray<HTMLElement>("[data-pill]");
      gsap.from(items, {
        x: 90,
        autoAlpha: 0,
        duration: 1.1,
        ease: "expo.out",
        stagger: 0.08,
        scrollTrigger: { trigger: root.current, start: "top 75%", once: true },
      });
      items.forEach((el, i) =>
        gsap.to(el.firstElementChild, { y: i % 2 ? 4 : -4, duration: 2.4 + i * 0.3, ease: "sine.inOut", yoyo: true, repeat: -1 }),
      );
    },
    { scope: root },
  );

  // A small pulse whenever the campaign changes.
  const first = useRef(true);
  useGSAP(
    () => {
      if (first.current || prefersReducedMotion()) {
        first.current = false;
        return;
      }
      gsap.fromTo("[data-pill-body]", { scale: 1 }, { scale: 1.035, duration: 0.18, yoyo: true, repeat: 1, ease: "power2.out", stagger: 0.03 });
    },
    { scope: root, dependencies: [total] },
  );

  return (
    <div ref={root} className="mt-16 grid items-center gap-10 overflow-hidden rounded-3xl bg-night p-7 text-white sm:p-10 lg:mt-24 lg:grid-cols-[1fr_auto] lg:p-14">
      <div>
        <p className="t-label text-white/70">/ What it could deliver</p>
        <h3 className="mt-5 max-w-[14ch] font-display text-[clamp(2rem,4vw,3.25rem)] leading-none tracking-[-0.03em]">
          Here’s what <span className="text-yellow">{naira(total)}</span> gets you.
        </h3>
        <p className="mt-5 max-w-[26rem] text-[1rem] leading-relaxed text-white/60">
          Change the niche, radius, format or views above and watch these move. No guesswork, before you spend a naira.
        </p>
        <p className="mt-6 text-[0.75rem] text-white/35">Sample estimates based on typical tap and visit rates.</p>
      </div>
      <ul className="flex flex-col gap-3 lg:w-[22rem]">
        {pills.map((p, i) => (
          <li key={p.label} data-pill className={pillOffsets[i]}>
            <div
              data-pill-body
              className={`flex items-center gap-4 rounded-full py-2 pl-2 pr-7 text-white shadow-[0_18px_40px_-18px_rgb(0_0_0/0.7)] ${pillStyles[i]}`}
            >
              <span className="block size-14 shrink-0 overflow-hidden rounded-full ring-2 ring-white/60">
                <Photo id={p.photo} w={112} h={112} />
              </span>
              <span className="leading-none">
                <span className="block font-display text-[1.75rem] tracking-[-0.03em]">
                  <Odometer value={p.value} />
                </span>
                <span className="mt-1 block text-[0.85rem] font-medium opacity-85">{p.label}</span>
              </span>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-3" style={{ borderColor: "var(--line)" }}>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="text-right font-medium">{children}</dd>
    </div>
  );
}

function Pickers() {
  const c = useCampaign();
  const formats: [Format, string][] = [
    ["banner", "Banner"],
    ["notification", "Notification"],
  ];
  const select =
    "h-11 w-full appearance-none rounded-xl border border-[var(--line-strong)] bg-white px-3.5 pr-9 text-[0.92rem] focus-visible:outline-2 focus-visible:outline-blue";
  return (
    // Format gets a full row wherever the column is too narrow for three across.
    <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-2 xl:grid-cols-3">
      <div className="sm:col-span-2 md:col-span-1 lg:col-span-2 xl:col-span-1">
        <p className="t-label text-faint">Format</p>
        <div role="radiogroup" aria-label="Format" className="mt-2 grid h-11 grid-cols-2 rounded-xl bg-stone p-1">
          {formats.map(([f, l]) => (
            <button
              key={f}
              type="button"
              role="radio"
              aria-checked={c.format === f}
              onClick={() => c.set("format", f)}
              className={`rounded-lg text-[0.85rem] font-medium transition-colors ${c.format === f ? "bg-white text-ink shadow-sm" : "text-muted-foreground hover:text-ink"}`}
            >
              {l}
            </button>
          ))}
        </div>
      </div>
      <label className="block">
        <span className="t-label text-faint">Niche</span>
        <span className="relative mt-2 block">
          <select className={select} value={c.niche} onChange={(e) => c.set("niche", e.target.value as typeof c.niche)}>
            {niches.map((n) => (
              <option key={n.id} value={n.id}>
                {n.label}
              </option>
            ))}
          </select>
          <Chevron />
        </span>
      </label>
      <label className="block">
        <span className="t-label text-faint">City</span>
        <span className="relative mt-2 block">
          <select className={select} value={c.city} onChange={(e) => c.set("city", e.target.value)}>
            {cities.map((x) => (
              <option key={x.id} value={x.id}>
                {x.name}
              </option>
            ))}
          </select>
          <Chevron />
        </span>
      </label>
    </div>
  );
}

function Chevron() {
  return (
    <svg viewBox="0 0 7 4" className="pointer-events-none absolute right-3.5 top-1/2 h-1 -translate-y-1/2 text-muted-foreground" shapeRendering="crispEdges" aria-hidden="true">
      <path d="M0 0h1v1H0zM1 1h1v1H1zM2 2h1v1H2zM3 3h1v1H3zM4 2h1v1H4zM5 1h1v1H5zM6 0h1v1H6z" fill="currentColor" />
    </svg>
  );
}
