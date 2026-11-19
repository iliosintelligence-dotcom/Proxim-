"use client";

import { useCallback, useRef, useState } from "react";
import { MapPin } from "lucide-react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import { advertiserCpm, impressionSteps, nicheById, type Format, type NicheId } from "@/lib/data";
import { commas, naira } from "@/lib/format";
import { Phone } from "@/components/phone/Phone";
import { AppScreen } from "@/components/phone/AppScreens";
import { Photo } from "@/components/phone/Photo";
import { Button } from "@/components/site/Button";
import { Frame } from "@/components/site/Frame";
import { Odometer } from "@/components/motion/Odometer";

/*
  The hero is the product in one loop. Six real-looking local businesses
  surround one phone. Every few seconds one of them sends its ad: a signal
  travels down its path, the phone opens the kind of app that business's
  customers use, and the ad lands. Nearby business, right app, ad seen.
  Hover or tap any business to send its ad yourself.
*/

type Spot = { id: NicheId; format: Format; x: number; y: number; area: string };

// Stage is laid out on a 1200 x 660 grid; cards are 196 wide.
const W = 1200;
const H = 660;
const CARD = 196;
const PHONE = { x: 455, y: 22, w: 290 };

const spots: Spot[] = [
  { id: "faith", format: "banner", x: 40, y: 34, area: "Lekki" },
  { id: "food", format: "banner", x: 214, y: 250, area: "Surulere" },
  { id: "education", format: "banner", x: 30, y: 446, area: "Bodija" },
  { id: "fitness", format: "notification", x: 964, y: 44, area: "Yaba" },
  { id: "music", format: "notification", x: 790, y: 256, area: "Wuse 2" },
  { id: "fashion", format: "banner", x: 974, y: 452, area: "GRA" },
];

const phoneY = [232, 330, 428, 240, 338, 436];

function pathFor(i: number) {
  const s = spots[i];
  const left = s.x < PHONE.x;
  const a = { x: left ? s.x + CARD : s.x, y: s.y + 104 };
  const p = { x: left ? PHONE.x + 8 : PHONE.x + PHONE.w - 8, y: phoneY[i] };
  const dx = (p.x - a.x) * 0.55;
  return `M${a.x},${a.y} C${a.x + dx},${a.y} ${p.x - dx},${p.y} ${p.x},${p.y}`;
}

export function Hero() {
  const root = useRef<HTMLElement>(null);
  const [active, setActive] = useState(0);
  const [shown, setShown] = useState(0);
  const [seen, setSeen] = useState(48_213);
  const lastManual = useRef(0);
  const spot = spots[shown];
  const niche = nicheById(spot.id);

  const { contextSafe } = useGSAP(
    () => {
      const reduce = prefersReducedMotion();
      const intro = gsap.utils.toArray<HTMLElement>("[data-intro]");
      if (reduce) {
        gsap.set(intro, { autoAlpha: 1 });
        return;
      }
      gsap.from(intro, { y: 18, autoAlpha: 0, duration: 1.1, stagger: 0.06, delay: 0.3 });
      gsap.from("[data-card-wrap]", {
        y: 26,
        scale: 0.94,
        autoAlpha: 0,
        duration: 1.2,
        stagger: { each: 0.08, from: "center" },
        delay: 0.55,
      });
      gsap.from("[data-hero-phone]", { y: 60, autoAlpha: 0, duration: 1.3, delay: 0.4 });
      gsap.utils.toArray<SVGPathElement>("[data-route]").forEach((p, i) => {
        const len = p.getTotalLength();
        gsap.fromTo(p, { strokeDashoffset: len }, { strokeDashoffset: 0, duration: 1.4, ease: "power2.inOut", delay: 0.9 + i * 0.06 });
      });

      // Leaving the hero pulls back from the neighbourhood: businesses
      // drift outward and the phone eases back.
      const stage = root.current!.querySelector("[data-stage-desktop]");
      if (stage) {
        const leave = gsap.timeline({
          scrollTrigger: { trigger: stage, start: "top 45%", end: "bottom top", scrub: 0.8 },
        });
        stage.querySelectorAll<HTMLElement>("[data-card-wrap]").forEach((w, i) => {
          const left = spots[i].x < PHONE.x;
          leave.to(w, { xPercent: left ? -22 : 22, yPercent: -10, ease: "none" }, 0);
        });
        leave.to(stage.querySelector("[data-hero-phone]"), { scale: 0.93, ease: "none" }, 0);
        leave.to(stage.querySelectorAll("[data-route]"), { opacity: 0.25, ease: "none" }, 0);
      }

      // The loop: the next business sends its ad every few seconds.
      let k = 0;
      const loop = gsap
        .timeline({ repeat: -1, delay: 3.4 })
        .call(() => {
          if (Date.now() - lastManual.current < 5000) return;
          k = (k + 1) % spots.length;
          send(k);
        })
        .to({}, { duration: 3.6 });
      ScrollTrigger.create({
        trigger: root.current,
        start: "top bottom",
        end: "bottom top",
        onToggle: (s) => (s.isActive ? loop.resume() : loop.pause()),
      });
    },
    { scope: root },
  );

  // Send business i's ad: light its route, run the signal, then land the ad.
  const send = contextSafe((i: number) => {
    setActive(i);
    const desktop = window.matchMedia("(min-width: 1024px)").matches;
    if (prefersReducedMotion() || !desktop) {
      gsap.delayedCall(0.2, () => setShown(i));
      return;
    }
    const hl = root.current!.querySelector<SVGPathElement>(`[data-hl="${i}"]`)!;
    const dot = root.current!.querySelector<SVGGElement>("[data-signal]")!;
    const len = hl.getTotalLength();
    gsap.killTweensOf([hl, dot]);
    gsap.set("[data-hl]", { opacity: 0 });
    gsap.fromTo(hl, { opacity: 1, strokeDasharray: len, strokeDashoffset: len }, { strokeDashoffset: 0, duration: 0.6, ease: "power2.out" });
    gsap.set(dot, { opacity: 1 });
    gsap.to(dot, {
      duration: 0.75,
      ease: "power1.inOut",
      motionPath: { path: hl, align: hl, alignOrigin: [0.5, 0.5] },
      onComplete: () => {
        gsap.to(dot, { opacity: 0, duration: 0.25 });
        setShown(i);
      },
    });
  });

  const pick = useCallback(
    (i: number) => {
      lastManual.current = Date.now();
      if (i !== active) send(i);
    },
    [active, send],
  );

  // When a new ad arrives, swap the app and land the ad.
  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      // Desktop and mobile each have their own phone; land the ad in both.
      root.current!.querySelectorAll("[data-hero-screen]").forEach((scr) => {
        gsap.fromTo(scr, { autoAlpha: 0.35, scale: 0.985 }, { autoAlpha: 1, scale: 1, duration: 0.5, ease: "power2.out" });
        const ad = scr.querySelector("[data-ad]");
        if (ad) {
          gsap.set(ad, { autoAlpha: 0 });
          adIn(ad, spot.format, shown === 0 ? 1.4 : 0.25);
        }
      });
      setSeen((s) => s + 6 + Math.floor(Math.random() * 18));
    },
    { scope: root, dependencies: [shown] },
  );

  return (
    <section ref={root} data-ad-manual className="relative overflow-hidden bg-paper">
      <Frame className="pt-[clamp(2.75rem,6.5vw,5rem)] text-center">
        <p data-intro className="t-label inline-flex items-center gap-2 text-ink">
          <MapPin className="size-3.5 text-blue" strokeWidth={2.2} />
          / Built for businesses across Nigeria
        </p>
        <h1 data-led className="t-display mx-auto mt-6 max-w-[18ch]">
          Put your ad where it belongs.
        </h1>
        <p data-intro className="t-lead mx-auto mt-5 max-w-[40rem]">
          Proxim places your banner or notification inside apps that match what you sell, and shows it only to people
          near you. A church invite in a Bible app. A free gym pass in a step counter.
        </p>
        <div data-intro className="mx-auto mt-8 grid w-full max-w-[20rem] gap-3 sm:flex sm:w-auto sm:max-w-none sm:justify-center">
          <Button href="#launch" arrow>
            Launch a campaign
          </Button>
          <Button href="#niche" variant="secondary">
            See how it works
          </Button>
        </div>
        <p data-intro className="t-label mt-5 text-faint">
          From {naira((impressionSteps[0] / 1000) * advertiserCpm.banner)} · No contracts · Pay only for views
        </p>
      </Frame>

      {/* Desktop: the neighbourhood around the phone */}
      <div data-stage-desktop className="relative mx-auto mt-14 hidden w-full max-w-[1200px] px-6 lg:block">
        <div className="relative w-full" style={{ aspectRatio: `${W} / ${H}` }}>
          <svg viewBox={`0 0 ${W} ${H}`} className="absolute inset-0 size-full overflow-visible" aria-hidden="true">
            {spots.map((_, i) => (
              <path
                key={i}
                data-route
                d={pathFor(i)}
                fill="none"
                stroke="rgb(28 25 23 / 0.16)"
                strokeWidth="1.2"
                strokeDasharray="3 5"
              />
            ))}
            {spots.map((_, i) => (
              <path key={`h${i}`} data-hl={i} d={pathFor(i)} fill="none" stroke="#0062ff" strokeWidth="1.6" opacity="0" />
            ))}
            <g data-signal opacity="0">
              <circle r="11" fill="#0062ff" opacity="0.18" />
              <circle r="4.5" fill="#0062ff" stroke="white" strokeWidth="1.5" />
            </g>
          </svg>

          {spots.map((s, i) => (
            <div
              key={s.id}
              data-card-wrap
              className="absolute"
              style={{ left: `${(s.x / W) * 100}%`, top: `${(s.y / H) * 100}%`, width: `${(CARD / W) * 100}%` }}
            >
              <BusinessCard spot={s} active={active === i} dim={active !== i} onPick={() => pick(i)} />
            </div>
          ))}

          <div
            data-hero-phone
            className="absolute"
            style={{ left: `${(PHONE.x / W) * 100}%`, top: `${(PHONE.y / H) * 100}%`, width: `${(PHONE.w / W) * 100}%` }}
          >
            <HeroPhone spot={spot} />
          </div>
        </div>
      </div>

      {/* Phone and tablet: the phone, then the businesses as a swipeable row */}
      <div className="mt-12 lg:hidden">
        <div data-hero-phone className="mx-auto w-[min(17rem,64vw)]">
          <HeroPhone spot={spot} />
        </div>
        <div className="mt-8 flex snap-x snap-mandatory gap-3 overflow-x-auto px-[var(--gutter)] pb-2 [scrollbar-width:none]">
          {spots.map((s, i) => (
            <div key={s.id} data-card-wrap className="w-[11.5rem] shrink-0 snap-center">
              <BusinessCard spot={s} active={active === i} onPick={() => pick(i)} />
            </div>
          ))}
        </div>
      </div>

      <div data-intro className="relative mx-auto mt-8 flex w-fit items-center gap-3 rounded-full border border-[var(--line)] bg-white px-4 py-2 text-[0.85rem] shadow-[0_1px_2px_rgb(0_0_0/0.04)]">
        <span className="relative flex size-2">
          <span className="absolute inset-0 animate-ping rounded-full bg-[#22c55e]/60" />
          <span className="relative size-2 rounded-full bg-[#22c55e]" />
        </span>
        <span className="text-muted-foreground">Views delivered today</span>
        <span className="font-display text-[1.05rem] tracking-[-0.02em]">
          <Odometer value={commas(seen)} />
        </span>
      </div>
      <div className="h-[clamp(3.5rem,7vw,5.5rem)]" />
      <span className="sr-only" aria-live="polite">
        {niche.ad.brand} ad showing in {niche.app.name}
      </span>
    </section>
  );
}

function HeroPhone({ spot }: { spot: Spot }) {
  const niche = nicheById(spot.id);
  return (
    <Phone label={`${niche.ad.brand} ${spot.format} ad in ${niche.app.name}`}>
      <div key={spot.id} data-hero-screen className="absolute inset-0">
        <AppScreen niche={niche} format={spot.format} />
      </div>
    </Phone>
  );
}

function BusinessCard({
  spot,
  active,
  dim = false,
  onPick,
}: {
  spot: Spot;
  active: boolean;
  dim?: boolean;
  onPick: () => void;
}) {
  const niche = nicheById(spot.id);
  return (
    <button
      type="button"
      onPointerEnter={(e) => e.pointerType === "mouse" && onPick()}
      onClick={onPick}
      aria-pressed={active}
      aria-label={`Send ${niche.ad.brand}'s ad`}
      className={`group block w-full rounded-2xl bg-white p-1.5 text-left ring-1 transition-[transform,box-shadow,opacity] duration-500 ease-[var(--ease-out-quint)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${
        active
          ? "-translate-y-1 shadow-[0_24px_48px_-20px_rgb(0_40_120/0.35)] ring-blue/50"
          : "shadow-[0_14px_34px_-18px_rgb(28_25_23/0.3)] ring-black/[0.06] hover:-translate-y-0.5"
      } ${dim ? "lg:opacity-80" : ""}`}
    >
      <div className="relative aspect-[4/3] overflow-hidden rounded-[13px]">
        <Photo id={niche.ad.photo} w={400} h={300} alt="" />
        <span className="absolute left-2 top-2 rounded-full bg-white/90 px-2 py-0.5 text-[11px] font-semibold text-ink backdrop-blur">
          {niche.kind}
        </span>
      </div>
      <div className="px-2 pb-1.5 pt-2">
        <p className="truncate text-[13.5px] font-semibold tracking-[-0.01em]">{niche.ad.brand}</p>
        <p className={`mt-0.5 flex items-center gap-1 truncate text-[12px] ${active ? "text-blue" : "text-muted-foreground"}`}>
          <MapPin className="size-3 shrink-0" />
          {active ? `Live in ${niche.app.name}` : `${spot.area} · ${niche.ad.distance}`}
        </p>
      </div>
    </button>
  );
}
