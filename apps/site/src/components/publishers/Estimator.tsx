"use client";

import { TrendingUp } from "lucide-react";
import { useState } from "react";
import type React from "react";
import { fillRate, publisherEcpm } from "@/lib/data";
import { commas, compact, naira } from "@/lib/format";
import { Section } from "@/components/site/Frame";
import { Odometer } from "@/components/motion/Odometer";
import { Switch } from "@/components/site/Switch";
import { HIcon } from "@/components/site/HIcon";

const dauSteps = [1_000, 2_500, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000];
const NOTIFS_PER_WEEK = 2;

/**
 * Mirrors the advertiser's pricing ticket from the other side: drag your
 * audience and watch a month's income roll up. Same sliders, same
 * odometer, same ticket shape, so the two pages read as one product.
 */
export function Estimator() {
  const [dauIndex, setDauIndex] = useState(4);
  const [sessions, setSessions] = useState(3);
  const [notifs, setNotifs] = useState(true);
  const dau = dauSteps[dauIndex];

  const bannerViews = dau * sessions * 30 * fillRate;
  const notifViews = notifs ? dau * (NOTIFS_PER_WEEK / 7) * 30 * fillRate : 0;
  const banner = (bannerViews / 1000) * publisherEcpm.banner;
  const notif = (notifViews / 1000) * publisherEcpm.notification;
  const monthly = banner + notif;
  const bannerShare = monthly ? (banner / monthly) * 100 : 100;

  return (
    <Section id="estimate" index="01" label="Earnings" icon={TrendingUp} pip="Slide to your numbers. That’s your monthly income.">
      <div className="grid gap-14 lg:grid-cols-[1fr_minmax(0,27rem)] lg:gap-20">
        <div>
          <h2 data-led className="t-section max-w-[18ch]">
            See what your <HIcon photo="1748002369513-af0f999d9f1e" /> audience is worth.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[33rem]">
            Every time a Proxim ad is seen in your app, you earn. Move the sliders to your numbers and see a month of
            income before you write a line of code.
          </p>

          <div data-reveal className="mt-12 space-y-10">
            <div>
              <div className="flex items-baseline justify-between">
                <label htmlFor="dau" className="t-label text-faint">
                  Daily active users
                </label>
                <span className="font-display text-[1.6rem] leading-none tracking-[-0.03em]">
                  <Odometer value={commas(dau)} />
                </span>
              </div>
              <input
                id="dau"
                type="range"
                min={0}
                max={dauSteps.length - 1}
                value={dauIndex}
                onChange={(e) => setDauIndex(Number(e.target.value))}
                aria-valuetext={`${commas(dau)} daily active users`}
                className="range mt-3"
                style={{ "--fill": `${(dauIndex / (dauSteps.length - 1)) * 100}%` } as React.CSSProperties}
              />
              <div className="mt-1 flex justify-between text-[0.75rem] text-faint">
                <span>{compact(dauSteps[0])}</span>
                <span>{compact(dauSteps[dauSteps.length - 1])}</span>
              </div>
            </div>

            <div>
              <div className="flex items-baseline justify-between">
                <label htmlFor="sessions" className="t-label text-faint">
                  Times each user opens your app a day
                </label>
                <span className="font-display text-[1.6rem] leading-none tracking-[-0.03em]">
                  <Odometer value={`${sessions}`} />
                </span>
              </div>
              <input
                id="sessions"
                type="range"
                min={1}
                max={8}
                value={sessions}
                onChange={(e) => setSessions(Number(e.target.value))}
                className="range mt-3"
                style={{ "--fill": `${((sessions - 1) / 7) * 100}%` } as React.CSSProperties}
              />
            </div>

            <div className="flex items-center justify-between gap-6 rounded-2xl border border-[var(--line)] bg-stone/60 p-4">
              <span>
                <span className="block font-medium">Also allow notification ads</span>
                <span className="block text-[0.85rem] text-muted-foreground">
                  Up to {NOTIFS_PER_WEEK} a week per user. They pay three times more.
                </span>
              </span>
              <Switch on={notifs} onChange={setNotifs} label="Allow notification ads" />
            </div>
          </div>
        </div>

        <div data-reveal className="lg:sticky lg:top-24 lg:self-start">
          <div className="overflow-hidden rounded-3xl border border-[var(--line)] bg-white shadow-[0_30px_60px_-30px_rgb(28_25_23/0.35)]">
            <div className="flex items-center justify-between px-6 pb-4 pt-6">
              <p className="font-display text-[1.2rem] tracking-[-0.02em]">Your estimate</p>
              <span className="t-label rounded-full bg-blue-soft px-2.5 py-1 text-blue">Per month</span>
            </div>
            <div className="px-6">
              <p className="font-display text-[clamp(2.4rem,5vw,3.2rem)] leading-none tracking-[-0.04em]">
                <Odometer value={naira(monthly)} />
              </p>
              <p className="mt-2 text-[0.88rem] text-muted-foreground">
                About <span className="font-medium text-ink">{naira(monthly * 12)}</span> a year
              </p>

              <div className="mt-6 flex h-2.5 overflow-hidden rounded-full bg-stone-2" aria-hidden="true">
                <span className="h-full bg-ink transition-[width] duration-700 ease-[var(--ease-out-quint)]" style={{ width: `${bannerShare}%` }} />
                <span className="h-full bg-blue transition-[width] duration-700 ease-[var(--ease-out-quint)]" style={{ width: `${100 - bannerShare}%` }} />
              </div>
              <dl className="mt-4 space-y-2 pb-6 text-[0.9rem]">
                <div className="flex justify-between">
                  <dt className="flex items-center gap-2 text-muted-foreground">
                    <span className="size-2 rounded-[2px] bg-ink" /> Banners · {compact(Math.round(bannerViews))} views
                  </dt>
                  <dd className="font-medium">{naira(banner)}</dd>
                </div>
                <div className={`flex justify-between transition-opacity ${notifs ? "" : "opacity-40"}`}>
                  <dt className="flex items-center gap-2 text-muted-foreground">
                    <span className="size-2 rounded-[2px] bg-blue" /> Notifications · {compact(Math.round(notifViews))} views
                  </dt>
                  <dd className="font-medium">{naira(notif)}</dd>
                </div>
              </dl>
            </div>
            <div className="perforation relative h-4">
              <span className="absolute -left-2 top-1/2 size-4 -translate-y-1/2 rounded-full bg-paper ring-1 ring-[var(--line)]" />
              <span className="absolute -right-2 top-1/2 size-4 -translate-y-1/2 rounded-full bg-paper ring-1 ring-[var(--line)]" />
            </div>
            <div className="bg-stone/50 px-6 pb-6 pt-5 text-[0.78rem] leading-relaxed text-faint">
              Sample rates: {naira(publisherEcpm.banner)} per 1,000 banner views, {naira(publisherEcpm.notification)} per
              1,000 notifications, {Math.round(fillRate * 100)}% of ad slots filled.
            </div>
          </div>
        </div>
      </div>
    </Section>
  );
}
