"use client";

import { Code } from "lucide-react";
import { useRef, useState } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Section } from "@/components/site/Frame";
import { HIcon } from "@/components/site/HIcon";

// Illustrative only: package names and APIs are placeholders until the SDK ships.
const samples = {
  Android: `// build.gradle
implementation("ng.proxim:sdk:1.0.0")

// MainActivity.kt
Proxim.start(this, appKey = "YOUR_APP_KEY")

val banner = ProximBanner(this, slot = "home_bottom")
layout.addView(banner)`,
  iOS: `// Swift Package: ProximSDK
import ProximSDK

Proxim.start(appKey: "YOUR_APP_KEY")

let banner = ProximBannerView(slot: "home_bottom")
view.addSubview(banner)`,
  Flutter: `// pubspec.yaml: proxim_ads: ^1.0.0
import 'package:proxim_ads/proxim_ads.dart';

await Proxim.start(appKey: 'YOUR_APP_KEY');

ProximBanner(slot: 'home_bottom')`,
  "React Native": `// npm i @proxim/react-native
import { Proxim, ProximBanner } from "@proxim/react-native";

Proxim.start({ appKey: "YOUR_APP_KEY" });

<ProximBanner slot="home_bottom" />`,
} as const;

type Lang = keyof typeof samples;

/** Tiny highlighter: comments, strings and a few keywords. */
function highlight(line: string) {
  if (/^\s*\/\//.test(line)) return <span className="text-white/35">{line}</span>;
  const parts = line.split(/("[^"]*"|'[^']*')/g);
  return parts.map((p, i) =>
    /^["']/.test(p) ? (
      <span key={i} className="text-ad-lime">
        {p}
      </span>
    ) : (
      <span key={i}>
        {p.split(/\b(import|from|val|let|await|implementation)\b/g).map((t, j) =>
          /^(import|from|val|let|await|implementation)$/.test(t) ? (
            <span key={j} className="text-ad-candy">
              {t}
            </span>
          ) : (
            t
          ),
        )}
      </span>
    ),
  );
}

const points = [
  ["Two calls", "Start the SDK, drop in a banner. Notifications need no extra code."],
  ["Looks like your app", "Corner radius, fonts and colours follow your theme."],
  ["Test mode first", "See sample ads in your build before a single real one runs."],
];

/**
 * The code doesn't type out character by character (slow, and fake). It
 * switches on line by line, in the same LED rhythm as the headlines.
 */
export function IntegrationSection() {
  const [lang, setLang] = useState<Lang>("Android");
  const code = useRef<HTMLPreElement>(null);
  const lines = samples[lang].split("\n");

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.fromTo(
        "[data-line]",
        { opacity: 0 },
        {
          keyframes: { opacity: [0, 1, 0.3, 1], easeEach: "steps(1)" },
          duration: 0.3,
          ease: "none",
          stagger: 0.05,
          scrollTrigger: { trigger: code.current, start: "top 85%", once: true },
        },
      );
    },
    { scope: code, dependencies: [lang], revertOnUpdate: true },
  );

  return (
    <Section id="integrate" index="03" label="Integration" tone="night" icon={Code} pip="Two calls and you’re live. Pick your platform.">
      <div className="grid gap-14 lg:grid-cols-[0.9fr_1.1fr] lg:gap-16">
        <div>
          <h2 data-led className="t-section max-w-[16ch]">
            Live in an <HIcon photo="1665849863678-fc03a642a9aa" /> afternoon.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[30rem] !text-white/55">
            Add the SDK, place a slot, and ads start filling. No ad server to run, no sales team to hire, no invoices
            to chase.
          </p>
          <ul className="mt-10 space-y-6">
            {points.map(([t, d], i) => (
              <li key={t} data-reveal className="flex gap-4">
                <span className="t-label mt-1 text-white/35">0{i + 1}</span>
                <span>
                  <span className="block font-medium">{t}</span>
                  <span className="block text-[0.92rem] text-white/50">{d}</span>
                </span>
              </li>
            ))}
          </ul>
        </div>

        <div data-reveal className="min-w-0 overflow-hidden rounded-3xl border border-white/10 bg-night-2">
          <div role="tablist" aria-label="Platform" className="flex gap-1 overflow-x-auto border-b border-white/10 p-2">
            {(Object.keys(samples) as Lang[]).map((l) => (
              <button
                key={l}
                type="button"
                role="tab"
                aria-selected={l === lang}
                onClick={() => setLang(l)}
                className={`shrink-0 rounded-full px-3.5 py-1.5 text-[0.84rem] transition-colors focus-visible:outline-2 focus-visible:outline-blue ${
                  l === lang ? "bg-white text-ink" : "text-white/55 hover:text-white"
                }`}
              >
                {l}
              </button>
            ))}
          </div>
          <pre
            ref={code}
            className="overflow-x-auto p-5 font-mono text-[0.8rem] leading-[1.85] text-white/85 sm:p-7 sm:text-[0.86rem]"
          >
            <code>
              {lines.map((l, i) => (
                <span key={`${lang}-${i}`} data-line className="block min-h-[1.85em]">
                  <span className="mr-5 inline-block w-4 select-none text-right text-white/20">{i + 1}</span>
                  {highlight(l)}
                </span>
              ))}
            </code>
          </pre>
          <p className="border-t border-white/10 px-5 py-3 text-[0.75rem] text-white/30 sm:px-7">
            Example code. Package names are placeholders.
          </p>
        </div>
      </div>
    </Section>
  );
}
