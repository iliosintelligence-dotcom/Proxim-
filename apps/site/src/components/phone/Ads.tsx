import { MapPin } from "lucide-react";
import type { Niche } from "@/lib/data";
import { tintBg } from "@/lib/data";
import { Photo } from "./Photo";
import { PipMark } from "@/components/mascot/PipShape";

/*
  The ad formats, measured in iOS points (see .phone-ui / .ios-scale).
  Every ad leads with a real photo and the advertiser's colour; the type
  is the same Geist as the rest of the system, so the brand comes from
  imagery and colour, the way a well-made native ad does.
*/

function BrandAvatar({ niche, size = 36 }: { niche: Niche; size?: number }) {
  return (
    <span
      className="grid shrink-0 place-items-center overflow-hidden rounded-full bg-ink font-semibold text-white ring-2 ring-white/70"
      style={{ width: `calc(var(--spacing) * ${size})`, height: `calc(var(--spacing) * ${size})`, fontSize: `calc(var(--spacing) * ${size * 0.36})` }}
    >
      {niche.ad.initials}
    </span>
  );
}

/** Native card ad: sits in an app's feed like the content around it. */
export function BannerAd({ niche, className = "" }: { niche: Niche; className?: string }) {
  const { ad } = niche;
  return (
    <div
      data-ad="banner"
      className={`overflow-hidden r-18 bg-white text-ink shadow-[0_1px_2px_rgb(0_0_0/0.08),0_12px_32px_-12px_rgb(0_0_0/0.4)] ring-1 ring-black/5 ${className}`}
    >
      <div className="relative h-148 overflow-hidden">
        <Photo id={ad.photo} w={720} h={300} />
        <span className="absolute left-10 top-10 rounded-full bg-white/90 px-8 py-3 fs-11 font-semibold tracking-[0.02em] text-black/70 backdrop-blur">
          Sponsored
        </span>
        <span className="absolute right-10 top-10 inline-flex items-center gap-3 rounded-full bg-black/55 px-8 py-3 fs-11 font-medium text-white backdrop-blur">
          <MapPin className="size-11" strokeWidth={2.4} />
          {ad.distance}
        </span>
      </div>
      <div className={`px-14 pb-14 pt-12 ${tintBg[ad.tint]}`}>
        <div className="flex items-center gap-8">
          <BrandAvatar niche={niche} size={30} />
          <div className="min-w-0 leading-tight">
            <p className="truncate fs-14 font-semibold">{ad.brand}</p>
            <p className="truncate fs-12 text-black/55">{ad.place}</p>
          </div>
        </div>
        <p className="mt-10 fs-20 font-semibold leading-[1.15] tracking-[-0.02em]">{ad.title}</p>
        <p className="mt-3 truncate fs-14 text-black/65">{ad.body}</p>
        <div className="mt-12 flex items-center justify-between">
          <span className="inline-flex h-34 items-center rounded-full bg-ink px-16 fs-14 font-semibold text-white">
            {ad.cta}
          </span>
          <span className="fs-11 font-medium text-black/45">Ad · Proxim</span>
        </div>
      </div>
    </div>
  );
}

/** Compact in-app banner for screens with less room (players, quizzes). */
export function CompactBannerAd({ niche, className = "" }: { niche: Niche; className?: string }) {
  const { ad } = niche;
  return (
    <div
      data-ad="banner"
      className={`flex items-center gap-12 r-16 p-8 pr-12 text-ink shadow-[0_1px_2px_rgb(0_0_0/0.08),0_12px_28px_-12px_rgb(0_0_0/0.45)] ring-1 ring-black/5 ${tintBg[ad.tint]} ${className}`}
    >
      <span className="size-56 shrink-0 overflow-hidden r-12">
        <Photo id={ad.photo} w={112} h={112} />
      </span>
      <div className="min-w-0 flex-1 leading-tight">
        <p className="truncate fs-12 font-medium text-black/55">
          {ad.brand} · {ad.distance}
        </p>
        <p className="mt-2 truncate fs-15 font-semibold tracking-[-0.01em]">{ad.title}</p>
        <p className="mt-2 fs-11 font-medium text-black/40">Sponsored</p>
      </div>
      <span className="inline-flex h-32 shrink-0 items-center rounded-full bg-ink px-12 fs-13 font-semibold text-white">
        {ad.cta.split(" ")[0]}
      </span>
    </div>
  );
}

/** iOS 17 notification, with the advertiser's photo as a rich attachment. */
export function NotificationAd({ niche, className = "" }: { niche: Niche; className?: string }) {
  const { ad } = niche;
  return (
    <div
      data-ad="notification"
      className={`flex items-center gap-10 r-22 bg-[rgb(245_245_247/0.8)] px-14 py-12 text-black shadow-[0_8px_30px_-8px_rgb(0_0_0/0.35)] backdrop-blur-2xl backdrop-saturate-150 ${className}`}
    >
      <span className={`grid size-38 shrink-0 place-items-center r-9 fs-14 font-bold text-ink ${tintBg[ad.tint]}`}>
        {ad.initials}
      </span>
      <div className="min-w-0 flex-1 leading-[1.28]">
        <div className="flex items-baseline justify-between gap-6">
          <p className="truncate fs-15 font-semibold">{ad.brand}</p>
          <p className="shrink-0 fs-13 text-black/45">now</p>
        </div>
        <p className="truncate fs-15">{ad.title}</p>
        <p className="truncate fs-13 text-black/50">Sponsored · {ad.distance} away</p>
      </div>
      <span className="size-40 shrink-0 overflow-hidden r-8">
        <Photo id={ad.photo} w={80} h={80} />
      </span>
    </div>
  );
}

/** A notification from Proxim itself (app owners page). */
export function ProximNote({ title, body, time = "now" }: { title: string; body: string; time?: string }) {
  return (
    <div className="flex gap-10 r-22 bg-[rgb(245_245_247/0.85)] px-14 py-12 text-black shadow-[0_8px_30px_-8px_rgb(0_0_0/0.35)] backdrop-blur-2xl">
      <span className="grid size-38 shrink-0 place-items-center r-9 bg-white ring-1 ring-black/5">
        <PipMark className="h-26 w-auto" cheeks={false} />
      </span>
      <div className="min-w-0 flex-1 leading-[1.28]">
        <div className="flex items-baseline justify-between gap-6">
          <p className="fs-15 font-semibold">{title}</p>
          <p className="shrink-0 fs-13 text-black/45">{time}</p>
        </div>
        <p className="fs-15 text-black/80">{body}</p>
      </div>
    </div>
  );
}
