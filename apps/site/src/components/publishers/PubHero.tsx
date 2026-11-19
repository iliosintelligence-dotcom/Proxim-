"use client";

import { Eye, MapPin, Plus, Smartphone } from "lucide-react";
import type React from "react";
import { useRef, useState } from "react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { adIn } from "@/lib/adMotion";
import { nicheById, publisherEcpm, type NicheId } from "@/lib/data";
import { naira } from "@/lib/format";
import { Phone } from "@/components/phone/Phone";
import { AppScreen } from "@/components/phone/AppScreens";
import { Button } from "@/components/site/Button";
import { Frame } from "@/components/site/Frame";
import { Odometer } from "@/components/motion/Odometer";
import { Photo } from "@/components/phone/Photo";

type Event = { id: number; brand: string; kind: "Banner view" | "Notification"; amount: number };

const rotation: NicheId[] = ["faith", "food", "education", "fitness"];

/**
 * The app owner's hero mirrors the advertiser's, seen from the other side:
 * one app (theirs) and a running ledger. Every time an ad lands in the
 * phone, a line of earnings is written and the month's total rolls up.
 * Ads become money in front of you.
 */
export function PubHero() {
  const root = useRef<HTMLElement>(null);
  const [adIndex, setAdIndex] = useState(0);
  const [total, setTotal] = useState(482_310);
  const [events, setEvents] = useState<Event[]>([
    { id: 3, brand: "Mama Titi’s Kitchen", kind: "Banner view", amount: 41 },
    { id: 2, brand: "Grace Assembly", kind: "Notification", amount: 108 },
    { id: 1, brand: "Bright Minds Tutorial", kind: "Banner view", amount: 83 },
  ]);
  const ad = nicheById(rotation[adIndex]);

  const earn = (brand: string) => {
    // A burst of views lands at once; per-view sums are sample rates.
    const views = 40 + Math.floor(Math.random() * 120);
    const amount = (views * publisherEcpm.banner) / 1000;
    setTotal((t) => t + amount);
    setEvents((e) => [{ id: Date.now(), brand, kind: "Banner view" as const, amount }, ...e].slice(0, 4));
  };

  useGSAP(
    () => {
      const intro = gsap.utils.toArray<HTMLElement>("[data-intro]");
      if (prefersReducedMotion()) {
        gsap.set(intro, { autoAlpha: 1 });
        return;
      }
      gsap.from(intro, { y: 24, autoAlpha: 0, duration: 1.2, stagger: 0.08, delay: 0.3 });

      // Stickers burst out of the top of the phone and settle in place.
      const stage = root.current!.querySelector<HTMLElement>("[data-sticker-stage]")!;
      const stickers = gsap.utils.toArray<HTMLElement>("[data-sticker]");
      const origin = { x: stage.offsetWidth / 2, y: stage.offsetHeight * 0.62 };
      stickers.forEach((el, i) => {
        const dx = origin.x - (el.offsetLeft + el.offsetWidth / 2);
        const dy = origin.y - (el.offsetTop + el.offsetHeight / 2);
        gsap.from(el, {
          x: dx,
          y: dy,
          scale: 0.2,
          autoAlpha: 0,
          duration: 1.1,
          ease: "back.out(1.5)",
          delay: 0.9 + i * 0.05,
        });
        gsap.to(el.querySelector("[data-sticker-float]"), {
          y: i % 2 ? 7 : -7,
          duration: 2.2 + (i % 4) * 0.4,
          ease: "sine.inOut",
          yoyo: true,
          repeat: -1,
          delay: i * 0.1,
        });
      });

      // The cloud leans toward the cursor and scatters as you scroll away.
      const cloud = root.current!.querySelector<HTMLElement>("[data-cloud]")!;
      const xTo = gsap.quickTo(cloud, "x", { duration: 1, ease: "expo.out" });
      const yTo = gsap.quickTo(cloud, "y", { duration: 1, ease: "expo.out" });
      const move = (e: PointerEvent) => {
        if (e.pointerType !== "mouse") return;
        xTo((e.clientX / window.innerWidth - 0.5) * 24);
        yTo((e.clientY / window.innerHeight - 0.5) * 14);
      };
      window.addEventListener("pointermove", move);
      gsap.to(stickers, {
        yPercent: (i) => -40 - (i % 3) * 25,
        xPercent: (i) => (i % 2 ? 1 : -1) * (10 + (i % 4) * 8),
        ease: "none",
        scrollTrigger: { trigger: stage, start: "top 30%", end: "bottom top", scrub: 0.8 },
      });
      const loop = gsap
        .timeline({ repeat: -1, delay: 2.6 })
        .call(() => setAdIndex((i) => (i + 1) % rotation.length))
        .to({}, { duration: 3.4 });
      ScrollTrigger.create({
        trigger: root.current,
        start: "top bottom",
        end: "bottom top",
        onToggle: (self) => (self.isActive ? loop.resume() : loop.pause()),
      });
      return () => window.removeEventListener("pointermove", move);
    },
    { scope: root },
  );

  // Each new ad lands, then gets paid for.
  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const el = root.current!.querySelector("[data-ad]");
      gsap.set(el, { autoAlpha: 0 });
      adIn(el, "banner", adIndex === 0 ? 1.1 : 0.1);
      const t = gsap.delayedCall(adIndex === 0 ? 1.6 : 0.6, () => earn(ad.ad.brand));
      return () => t.kill();
    },
    { scope: root, dependencies: [adIndex] },
  );

  // Newest ledger line slides in on top.
  useGSAP(
    () => {
      if (prefersReducedMotion() || events[0].id < 10) return;
      gsap.from("[data-ledger] li:first-child", { y: -12, autoAlpha: 0, duration: 0.7 });
      gsap.fromTo("[data-earn-pop]", { scale: 1 }, { scale: 1.25, duration: 0.18, yoyo: true, repeat: 1, ease: "power2.out" });
    },
    { scope: root, dependencies: [events[0].id] },
  );

  return (
    <section ref={root} data-ad-manual className="relative overflow-hidden bg-paper">
      <Frame className="pt-[clamp(2.75rem,6.5vw,5rem)] text-center">
        <p data-intro className="t-label inline-flex items-center gap-2 text-ink">
          <Smartphone className="size-3.5 text-blue" strokeWidth={2.2} />
          / For app owners and developers
        </p>
        <h1 data-led className="t-display mx-auto mt-6 max-w-[19ch]">
          Get paid for the attention you already have.
        </h1>
        <p data-intro className="t-lead mx-auto mt-5 max-w-[40rem]">
          Add Proxim to your app and earn from local ads that fit your users. You decide which brands appear and how
          often. Your users get offers from businesses near them that they actually want.
        </p>
        <div data-intro className="mx-auto mt-8 grid w-full max-w-[20rem] gap-3 sm:flex sm:w-auto sm:max-w-none sm:justify-center">
          <Button href="#start" arrow>
            Start earning
          </Button>
          <Button href="#estimate" variant="secondary">
            Estimate your earnings
          </Button>
        </div>
        <p data-intro className="t-label mt-5 text-faint">
          Free to join · Paid monthly in naira · Live in an afternoon
        </p>
      </Frame>

      {/* The phone, held up from the bottom edge, with its attention bursting out as stickers. */}
      <div data-sticker-stage className="relative mx-auto mt-[clamp(2rem,4vw,3rem)] h-[clamp(36rem,64vw,46rem)] max-w-[1180px] overflow-hidden px-[var(--gutter)]">
        <div data-cloud className="absolute inset-0">
          {/* Earned this month */}
          <Sticker x="3%" y="14%" r={-4} className="hidden w-[17rem] md:block">
            <div className="rounded-2xl bg-white p-4 text-left shadow-[0_18px_40px_-20px_rgb(13_13_13/0.35)] ring-1 ring-black/5">
              <p className="t-label text-faint">Earned this month</p>
              <p className="mt-2 font-display text-[1.9rem] leading-none tracking-[-0.04em]">
                <Odometer value={naira(total)} />
              </p>
              <div className="mt-4 flex h-10 items-end gap-[3px]" aria-hidden="true">
                {Array.from({ length: 20 }, (_, i) => (
                  <span key={i} className={`flex-1 rounded-[1px] ${i === 19 ? "bg-blue" : "bg-stone-2"}`} style={{ height: `${30 + ((i * 37) % 50) + i}%` }} />
                ))}
              </div>
            </div>
          </Sticker>
          {/* Live ledger */}
          <Sticker x="72%" y="22%" r={3} className="hidden w-[16rem] lg:block">
            <div className="rounded-2xl bg-white p-4 text-left shadow-[0_18px_40px_-20px_rgb(13_13_13/0.35)] ring-1 ring-black/5">
              <p className="t-label text-faint">Live · Daily Word</p>
              <ul data-ledger className="mt-2 divide-y" style={{ borderColor: "var(--line)" }}>
                {events.slice(0, 3).map((e) => (
                  <li key={e.id} className="flex items-center justify-between gap-3 py-2 text-[0.82rem]" style={{ borderColor: "var(--line)" }}>
                    <span className="truncate font-medium">{e.brand}</span>
                    <span className="shrink-0 font-medium text-blue">+{naira(e.amount)}</span>
                  </li>
                ))}
              </ul>
            </div>
          </Sticker>

          <Sticker x="31%" y="10%" r={-8}><Pill className="bg-[#ff5b1f] text-white"><MapPin className="size-3.5" /> Lekki</Pill></Sticker>
          <Sticker x="44%" y="3%" r={5}>
            <span data-earn-pop className="block"><Pill className="bg-yellow text-ink">+{naira(events[0]?.amount ?? 41)}</Pill></span>
          </Sticker>
          <Sticker x="58%" y="9%" r={-3}><Pill className="bg-ink text-white"><Eye className="size-3.5" /> 312 views</Pill></Sticker>
          <Sticker x="24%" y="25%" r={6} className="hidden sm:block">
            <span className="flex items-center gap-2 rounded-full bg-white py-1 pl-1 pr-3 text-[0.82rem] font-medium shadow-[0_10px_24px_-12px_rgb(13_13_13/0.4)] ring-1 ring-black/5">
              <span className="block size-7 overflow-hidden rounded-full"><Photo id="1698827623494-c4e1196ba0ca" w={56} h={56} /></span>
              Mama Titi’s Kitchen
            </span>
          </Sticker>
          <Sticker x="66%" y="3%" r={10}>
            <span className="grid size-12 place-items-center rounded-full bg-yellow font-display text-[1.3rem] text-ink shadow-[0_10px_24px_-12px_rgb(13_13_13/0.45)] ring-2 ring-white">₦</span>
          </Sticker>
          <Sticker x="38%" y="24%" r={-5}><Pill className="bg-ad-lilac text-ink">Faith ads</Pill></Sticker>
          <Sticker x="61%" y="24%" r={7}><Pill className="bg-ad-candy text-ink"><MapPin className="size-3.5" /> Yaba</Pill></Sticker>
          <Sticker x="51%" y="17%" r={0} className="hidden sm:block">
            <span className="grid size-10 place-items-center rounded-full bg-ink text-white shadow-[0_10px_24px_-12px_rgb(13_13_13/0.45)]"><Plus className="size-5" /></span>
          </Sticker>
          <Sticker x="20%" y="7%" r={-6}><Pill className="bg-ad-lime text-ink">Food</Pill></Sticker>
          <Sticker x="75%" y="8%" r={-4} className="hidden sm:block">
            <span className="block size-14 overflow-hidden rounded-full shadow-[0_10px_24px_-12px_rgb(13_13_13/0.45)] ring-[3px] ring-white"><Photo id="1687422809069-0fa3546b8471" w={112} h={112} /></span>
          </Sticker>
          <Sticker x="33%" y="36%" r={4} className="hidden sm:block"><Pill className="bg-blue text-white">Banner</Pill></Sticker>
          <Sticker x="57%" y="35%" r={-7} className="hidden sm:block"><Pill className="bg-white text-ink ring-1 ring-black/10">Notification</Pill></Sticker>
        </div>

        <div className="absolute bottom-0 left-1/2 w-[min(17.5rem,64vw)] -translate-x-1/2 translate-y-[30%]">
          <div data-intro>
            <Phone label={`Daily Word app showing a ${ad.ad.brand} banner`}>
              <AppScreen niche={ad} host="faith" format="banner" />
            </Phone>
          </div>
        </div>
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-paper to-transparent" />
      </div>
    </section>
  );
}

function Sticker({ x, y, r = 0, className = "", children }: { x: string; y: string; r?: number; className?: string; children: React.ReactNode }) {
  return (
    <div data-sticker className={`absolute ${className}`} style={{ left: x, top: y, rotate: `${r}deg` }}>
      <div data-sticker-float>{children}</div>
    </div>
  );
}

function Pill({ className = "", children }: { className?: string; children: React.ReactNode }) {
  return (
    <span className={`inline-flex items-center gap-1.5 whitespace-nowrap rounded-full px-3.5 py-2 text-[0.85rem] font-semibold shadow-[0_10px_24px_-12px_rgb(13_13_13/0.45)] ${className}`}>
      {children}
    </span>
  );
}
