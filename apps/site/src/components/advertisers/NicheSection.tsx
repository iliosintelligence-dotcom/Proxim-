"use client";

import { Target } from "lucide-react";
import { useRef } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { NicheIcon } from "@/components/site/NicheIcon";
import { niches, nicheById } from "@/lib/data";
import { Section } from "@/components/site/Frame";
import { LivePhone } from "@/components/phone/LivePhone";
import { useCampaign } from "./CampaignContext";
import { HIcon } from "@/components/site/HIcon";

export function NicheSection() {
  const { niche, format, set } = useCampaign();
  const active = nicheById(niche);
  const list = useRef<HTMLDivElement>(null);

  // The chosen niche's icon switches on like a pixel sign.
  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.fromTo(
        '[aria-checked="true"] [data-niche-icon] img',
        { opacity: 0 },
        { keyframes: { opacity: [0, 1, 0.2, 1], easeEach: "steps(1)" }, duration: 0.35, ease: "none" },
      );
      gsap.fromTo('[aria-checked="true"] [data-niche-icon]', { scale: 0.85 }, { scale: 1, duration: 0.6, ease: "back.out(3)" });
    },
    { scope: list, dependencies: [niche], lazy: true },
  );

  return (
    <Section id="niche" index="01" label="The right app" icon={Target} pip="Pick what you sell. I’ll find the apps your customers use.">
      <div className="grid gap-14 lg:grid-cols-[1.05fr_1fr] lg:gap-20">
        <div>
          <h2 data-led className="t-section max-w-[20ch]">
            Show up where the <HIcon photo="1687422809654-579d81c29d32" /> interest already is.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[34rem]">
            Most ad networks show your church invite to football fans and your gym pass to people reading recipes.
            Proxim only places your ad in apps that fit what you sell, so the person who sees it is already halfway to
            yes.
          </p>

          <div ref={list} data-reveal role="radiogroup" aria-label="Choose a niche" className="mt-10 grid gap-2 sm:grid-cols-2">
            {niches.map((n) => {
              const on = n.id === niche;
              return (
                <button
                  key={n.id}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  onClick={() => set("niche", n.id)}
                  className={`group flex items-center gap-3.5 rounded-2xl border py-3 pl-3 pr-4 text-left transition-[background-color,border-color,box-shadow] duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${
                    on
                      ? "border-ink bg-white shadow-[0_6px_20px_-10px_rgb(28_25_23/0.35)]"
                      : "border-[var(--line)] bg-stone/60 hover:border-[var(--line-strong)] hover:bg-white"
                  }`}
                >
                  <NicheIcon
                    niche={n}
                    className="transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-0.5 group-hover:-rotate-3"
                  />
                  <span className="min-w-0">
                    <span className="block font-medium">{n.label}</span>
                    <span className="block truncate text-[0.82rem] text-muted-foreground">{n.hosts}</span>
                  </span>
                </button>
              );
            })}
          </div>
        </div>

        <div className="relative flex flex-col items-center">
          <div className="absolute inset-x-0 top-[8%] bottom-[8%] -z-0 rounded-3xl bg-stone" aria-hidden="true" />
          <div data-float className="relative">
            <LivePhone niche={active} format={format} className="relative w-[min(17rem,72vw)] py-10" />
          </div>
          <figure data-reveal className="relative -mt-4 w-full max-w-[22rem] rounded-2xl border border-[var(--line)] bg-white p-5 shadow-[0_10px_30px_-16px_rgb(28_25_23/0.3)]">
            <figcaption className="t-label text-faint">Why it works</figcaption>
            <p key={active.id} className="mt-2 text-[0.98rem] leading-relaxed text-ink-2">
              {active.why}
            </p>
          </figure>
        </div>
      </div>
    </Section>
  );
}
