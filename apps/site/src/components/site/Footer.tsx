"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { ArrowUp, Plus } from "lucide-react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { glideToTop } from "@/lib/lenis";
import { Photo } from "@/components/phone/Photo";
import { Logo } from "./Logo";
import { Pip } from "@/components/mascot/Pip";

const cols = [
  {
    title: "Product",
    links: [
      { label: "Features", href: "/features" },
      { label: "Pricing", href: "/pricing" },
      { label: "Ad formats", href: "/features#formats" },
      { label: "Targeting", href: "/features#targeting" },
    ],
  },
  {
    title: "App owners",
    links: [
      { label: "Earn with Proxim", href: "/publishers" },
      { label: "Earnings calculator", href: "/publishers#estimate" },
      { label: "SDK and docs", href: "/publishers#integrate" },
      { label: "Payouts", href: "/publishers#payouts" },
    ],
  },
  {
    title: "Company",
    links: [
      { label: "About", href: "/about" },
      { label: "Contact", href: "/contact" },
      { label: "Privacy", href: "#" },
      { label: "Terms", href: "#" },
    ],
  },
];

const owners = [
  "1698827623494-c4e1196ba0ca",
  "1687422809069-0fa3546b8471",
  "1687422808248-f807f4ea2a2e",
  "1702236240794-58dc4c6895e5",
  "1687422808311-a776f467a468",
];

/**
 * The footer sits on the blue the finale leaves behind, as a white card.
 * It arrives instead of just being there: the card rises and settles,
 * the wordmark rises letter by letter, and Pip keeps pinging beside it,
 * the product's idea one last time.
 */
export function Footer() {
  const root = useRef<HTMLElement>(null);
  const [time, setTime] = useState<string | null>(null);
  const [pings, setPings] = useState(0);

  useEffect(() => {
    const fmt = new Intl.DateTimeFormat("en-NG", { timeZone: "Africa/Lagos", hour: "2-digit", minute: "2-digit", hour12: false });
    const tick = () => setTime(fmt.format(new Date()));
    tick();
    const id = window.setInterval(tick, 20_000);
    return () => window.clearInterval(id);
  }, []);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const q = gsap.utils.selector(root);
      gsap.fromTo(
        q("[data-footer-card]"),
        { y: 120, scale: 0.94, borderRadius: 48 },
        {
          y: 0,
          scale: 1,
          borderRadius: 24,
          ease: "none",
          // Settles exactly at the bottom of the page, so it always finishes
          // expanding, however tall the screen is.
          scrollTrigger: { trigger: root.current, start: "top bottom", end: "bottom bottom", scrub: 0.8 },
        },
      );
      gsap.from(q("[data-footer-col]"), {
        y: 24,
        autoAlpha: 0,
        duration: 0.9,
        ease: "expo.out",
        stagger: 0.07,
        scrollTrigger: { trigger: q("[data-footer-card]")[0], start: "top 75%", once: true },
      });
      gsap.from(q("[data-wm-letter]"), {
        yPercent: 105,
        duration: 1.2,
        ease: "expo.out",
        stagger: 0.06,
        scrollTrigger: { trigger: q("[data-wordmark]")[0], start: "top 92%", once: true },
      });
      // Pip pings every few seconds while the footer is in view.
      const ping = gsap
        .timeline({ repeat: -1, paused: true })
        .call(() => setPings((n) => n + 1))
        .to({}, { duration: 2.6 });
      ScrollTrigger.create({ trigger: root.current, start: "top bottom", onToggle: (s) => (s.isActive ? ping.play() : ping.pause()) });
    },
    { scope: root, lazy: true },
  );

  return (
    <footer ref={root} className="overflow-hidden bg-blue p-2 sm:p-3">
      <div data-footer-card className="mx-auto max-w-[1400px] origin-bottom rounded-3xl bg-white px-[var(--gutter)] pt-10 text-ink sm:pt-12">
        <div className="grid gap-10 md:grid-cols-[1.35fr_repeat(3,1fr)]">
          {/* Community card */}
          <div data-footer-col className="rounded-2xl bg-stone p-6">
            <Logo />
            <p className="mt-5 max-w-[18ch] font-display text-[1.6rem] leading-[1.05] tracking-[-0.02em]">
              Join the businesses getting found nearby
            </p>
            <Link href="/#launch" className="group mt-5 flex items-center" aria-label="Start your first campaign">
              <span className="flex -space-x-3 transition-[margin] duration-500 ease-[var(--ease-out-quint)] group-hover:-space-x-1">
                {owners.map((id) => (
                  <span key={id} className="block size-11 overflow-hidden rounded-full ring-[3px] ring-stone transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-0.5">
                    <Photo id={id} w={88} h={88} />
                  </span>
                ))}
              </span>
              <span className="ml-2 grid size-11 place-items-center rounded-full bg-blue text-white ring-[3px] ring-stone transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:rotate-90">
                <Plus className="size-5" />
              </span>
            </Link>
            <p className="mt-3 text-[0.8rem] text-muted-foreground">Free to start · No card needed</p>
          </div>

          {cols.map((c) => (
            <div key={c.title} data-footer-col>
              <p className="text-[0.9rem] font-medium">{c.title}</p>
              <ul className="mt-3 space-y-2">
                {c.links.map((l) => (
                  <li key={l.label}>
                    <Link href={l.href} className="group inline-flex items-center gap-1 text-[0.9rem] text-muted-foreground transition-colors hover:text-ink">
                      <span className="h-px w-0 bg-ink transition-[width] duration-300 ease-[var(--ease-out-quint)] group-hover:w-3" />
                      {l.label}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        {/* Giant wordmark with the radar ring */}
        <div data-wordmark className="mt-14 flex items-end justify-between gap-4 border-t pt-6" style={{ borderColor: "var(--line)" }}>
          <p aria-label="proxim" className="flex overflow-hidden font-display text-[clamp(3.5rem,17vw,19rem)] leading-[0.8] tracking-[-0.05em]">
            {"proxim".split("").map((ch, i) => (
              <span key={i} aria-hidden="true" className="inline-block pb-[0.06em] transition-transform duration-500 ease-[var(--ease-out-quint)] hover:-translate-y-[0.08em]">
                <span data-wm-letter className="inline-block">
                  {ch}
                </span>
              </span>
            ))}
          </p>
          <div className="mb-[1vw] shrink-0">
            <Pip size={96} pingKey={pings} className="origin-bottom-right scale-[0.6] sm:scale-100" title="Pip" />
          </div>
        </div>

        <div className="flex flex-col items-start justify-between gap-3 border-t py-5 text-[0.8rem] text-muted-foreground sm:flex-row sm:items-center" style={{ borderColor: "var(--line)" }}>
          <span>© {new Date().getFullYear()} Proxim. Made for businesses across Nigeria.</span>
          <span className="inline-flex items-center gap-2">
            <span className="relative flex size-2">
              <span className="absolute inset-0 animate-ping rounded-full bg-[#22c55e]/60" />
              <span className="relative size-2 rounded-full bg-[#22c55e]" />
            </span>
            Lagos {time ?? "--:--"} WAT
          </span>
          <button
            type="button"
            onClick={glideToTop}
            className="group inline-flex items-center gap-2 rounded-lg bg-stone px-3 py-2 font-medium text-ink transition-colors hover:bg-stone-2"
          >
            Back to top
            <ArrowUp className="size-4 transition-transform duration-300 ease-[var(--ease-out-quint)] group-hover:-translate-y-0.5" />
          </button>
        </div>
      </div>
    </footer>
  );
}
