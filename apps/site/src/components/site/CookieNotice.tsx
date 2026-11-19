"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Pip } from "@/components/mascot/Pip";
import { RollingLabel, TriangleArrow } from "./Button";

const KEY = "proxim-cookies";

/**
 * Cookie notice, designed as part of the system rather than bolted on:
 * a white card that lands like one of the site's notifications, with Pip.
 * "Only essential" sits beside "Accept all" with equal weight, so saying
 * no is as easy as saying yes. The choice is remembered in this browser.
 */
export function CookieNotice() {
  const root = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let saved: string | null = null;
    try {
      saved = localStorage.getItem(KEY);
    } catch {
      /* storage blocked: show the notice each visit */
    }
    if (saved) return;
    const id = window.setTimeout(() => setOpen(true), 1400);
    return () => window.clearTimeout(id);
  }, []);

  useEffect(() => {
    window.dispatchEvent(new Event(open ? "cookies:open" : "cookies:close"));
  }, [open]);

  useGSAP(
    () => {
      if (!open) return;
      gsap.fromTo(
        root.current,
        { y: 40, autoAlpha: 0, scale: 0.96 },
        { y: 0, autoAlpha: 1, scale: 1, duration: prefersReducedMotion() ? 0 : 0.8, ease: "back.out(1.6)" },
      );
    },
    { dependencies: [open] },
  );

  const choose = (value: "all" | "essential") => {
    try {
      localStorage.setItem(KEY, value);
    } catch {
      /* ignore */
    }
    gsap.to(root.current, {
      y: 30,
      autoAlpha: 0,
      duration: prefersReducedMotion() ? 0 : 0.35,
      ease: "power2.in",
      onComplete: () => setOpen(false),
    });
  };

  if (!open) return null;

  return (
    <div
      ref={root}
      role="dialog"
      aria-live="polite"
      aria-label="Cookie preferences"
      className="invisible fixed inset-x-3 bottom-3 z-[60] rounded-2xl bg-white p-4 text-ink shadow-[0_0_0_1px_var(--line),0_24px_60px_-20px_rgb(13_13_13/0.45)] sm:inset-x-auto sm:bottom-6 sm:left-6 sm:w-[23rem] sm:p-5"
    >
      <div className="flex gap-3.5">
        <div className="-mt-1 shrink-0">
          <Pip size={40} title="Pip" />
        </div>
        <div className="min-w-0">
          <p className="font-display text-[1.2rem] leading-tight tracking-[-0.02em]">We use a few cookies</p>
          <p className="mt-1.5 text-[0.86rem] leading-relaxed text-muted-foreground">
            Essential ones keep the site working. With your OK, we also use analytics to see which pages help
            businesses most. No selling, no ads tracking you around.
          </p>
        </div>
      </div>
      <div className="mt-4 grid grid-cols-2 gap-2">
        <button
          type="button"
          onClick={() => choose("essential")}
          className="group h-10 rounded-lg bg-stone-2 px-3 text-[0.88rem] font-medium transition-colors hover:bg-[#d6d6d6] focus-visible:outline-2 focus-visible:outline-blue"
        >
          <span className="flex justify-center">
            <RollingLabel>Only essential</RollingLabel>
          </span>
        </button>
        <button
          type="button"
          onClick={() => choose("all")}
          className="group flex h-10 items-center justify-between gap-2 rounded-lg bg-ink pl-4 pr-1.5 text-[0.88rem] font-medium text-white transition-colors hover:bg-[#262626] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue"
        >
          <RollingLabel>Accept all</RollingLabel>
          <span className="grid size-7 place-items-center rounded-md bg-yellow text-ink transition-transform duration-300 group-hover:translate-x-0.5">
            <TriangleArrow />
          </span>
        </button>
      </div>
      <Link href="#" className="mt-3 inline-block text-[0.78rem] text-muted-foreground underline-offset-4 hover:text-ink hover:underline">
        Cookie policy
      </Link>
    </div>
  );
}
