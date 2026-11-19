import {
  Activity,
  Bookmark,
  BookOpen,
  Camera,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Clock,
  Compass,
  Dumbbell,
  Ellipsis,
  Flame,
  Flashlight,
  Heart,
  House,
  ListChecks,
  ListMusic,
  Lock,
  Menu,
  MessageCircle,
  Pause,
  Plus,
  Repeat2,
  Search,
  Share,
  Share2,
  Shuffle,
  SkipBack,
  SkipForward,
  Speaker,
  Star,
  Users,
  X,
  type LucideIcon,
} from "lucide-react";
import type { Format, Niche, NicheId } from "@/lib/data";
import { appPhotos } from "@/lib/data";
import { StatusBar, HomeIndicator } from "./Phone";
import { BannerAd, CompactBannerAd, NotificationAd } from "./Ads";
import { Photo } from "./Photo";

/*
  Six made-up host apps, each modelled on how the leader in its category
  is built (a scripture app, an activity tracker, a recipe app, an exam
  trainer, a music player, a style board), at Apple's real sizes. All
  measurements are iOS points: p-16 is 16pt, fs-17 is 17pt.
*/

type Tab = [LucideIcon, string];

function TabBar({ tabs, active = 0, accent, dark = false, labels = true }: { tabs: Tab[]; active?: number; accent: string; dark?: boolean; labels?: boolean }) {
  return (
    <div
      className={`absolute inset-x-0 bottom-0 z-20 h-83 border-t backdrop-blur-xl ${dark ? "border-white/10 bg-[rgb(18_18_18/0.92)]" : "border-black/10 bg-[rgb(250_250_250/0.92)]"}`}
    >
      <div className="grid pt-7" style={{ gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}>
        {tabs.map(([Icon, label], i) => (
          <div
            key={label}
            className="flex flex-col items-center gap-2"
            style={{ color: i === active ? accent : dark ? "rgb(255 255 255 / 0.45)" : "rgb(0 0 0 / 0.38)" }}
          >
            <Icon className="size-25" strokeWidth={i === active ? 2.3 : 1.8} />
            {labels && <span className="fs-10 font-medium tracking-[0.01em]">{label}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}

function Avatar({ id, size = 32 }: { id: string; size?: number }) {
  return (
    <span
      className="block shrink-0 overflow-hidden rounded-full ring-1 ring-black/10"
      style={{ width: `calc(var(--spacing) * ${size})`, height: `calc(var(--spacing) * ${size})` }}
    >
      <Photo id={id} w={size * 3} h={size * 3} />
    </span>
  );
}

/* ---------------- Daily Word: scripture ---------------- */

const FAITH = "#c2410c";

function BibleApp() {
  const days = [["M", 10], ["T", 11], ["W", 12], ["T", 13], ["F", 14], ["S", 15], ["S", 16]] as const;
  return (
    <div className="absolute inset-0 bg-white">
      <StatusBar />
      <div className="flex items-center justify-between px-20 pt-2">
        <p className="fs-34 font-bold tracking-[-0.03em]">Today</p>
        <div className="flex items-center gap-10">
          <span className="inline-flex h-30 items-center gap-4 rounded-full px-10 fs-14 font-semibold" style={{ background: "#fff1e8", color: FAITH }}>
            <Flame className="size-15" strokeWidth={2.4} />
            12
          </span>
          <Avatar id="1760907949889-eb62b7fd9f75" size={32} />
        </div>
      </div>
      <div className="mt-14 grid grid-cols-7 px-14 text-center">
        {days.map(([d, n], i) => (
          <div key={i} className="flex flex-col items-center gap-5">
            <span className="fs-11 font-medium text-black/40">{d}</span>
            <span
              className={`grid size-32 place-items-center rounded-full fs-15 font-semibold ${i === 3 ? "text-white" : i < 3 ? "text-black" : "text-black/35"}`}
              style={i === 3 ? { background: FAITH } : undefined}
            >
              {n}
            </span>
            <span className="size-4 rounded-full" style={{ background: i < 3 ? FAITH : "transparent" }} />
          </div>
        ))}
      </div>
      <div className="relative mx-16 mt-12 h-306 overflow-hidden r-18">
        <Photo id={appPhotos.verse} w={720} h={612} />
        <div className="absolute inset-0 bg-gradient-to-t from-black/75 via-black/20 to-black/10" />
        <p className="absolute left-16 top-14 fs-12 font-semibold uppercase tracking-[0.08em] text-white/85">Verse of the Day</p>
        <div className="absolute inset-x-16 bottom-14 text-white">
          <p className="fs-22 font-semibold leading-[1.2] tracking-[-0.02em]">
            “For God so loved the world, that he gave his only begotten Son.”
          </p>
          <div className="mt-10 flex items-center justify-between">
            <p className="fs-13 text-white/75">John 3:16 · KJV</p>
            <div className="flex items-center gap-14 fs-13 text-white/90">
              <span className="inline-flex items-center gap-4"><Heart className="size-17" /> 2.4K</span>
              <Share className="size-17" />
              <Bookmark className="size-17" />
            </div>
          </div>
        </div>
      </div>
      <div className="mx-16 mt-12 flex items-center gap-12 r-14 bg-[#f6f5f3] p-12">
        <span className="grid size-44 place-items-center r-10 bg-white" style={{ color: FAITH }}>
          <BookOpen className="size-22" />
        </span>
        <div className="min-w-0 flex-1 leading-tight">
          <p className="fs-15 font-semibold">Guided Prayer</p>
          <p className="fs-13 text-black/50">Morning · 6 min</p>
        </div>
        <ChevronRight className="size-18 text-black/30" />
      </div>
      <TabBar accent={FAITH} tabs={[[House, "Home"], [BookOpen, "Bible"], [ListChecks, "Plans"], [Compass, "Discover"], [Menu, "More"]]} />
    </div>
  );
}

/* ---------------- Stride: activity ---------------- */

function FitnessApp() {
  const rings = [
    { r: 46, c: "#ff375f", p: 0.78 },
    { r: 34, c: "#9cf02c", p: 0.73 },
    { r: 22, c: "#2fe3f8", p: 0.75 },
  ];
  return (
    <div className="absolute inset-0 bg-black text-white">
      <StatusBar light />
      <div className="flex items-end justify-between px-16 pt-2">
        <div className="leading-tight">
          <p className="fs-13 font-semibold uppercase tracking-[0.02em] text-white/50">Thursday, 13 Nov</p>
          <p className="fs-34 font-bold tracking-[-0.03em]">Summary</p>
        </div>
        <Avatar id="1649887974297-4be052375a67" size={34} />
      </div>
      <div className="mx-16 mt-14 r-14 bg-[#1c1c1e] p-16">
        <p className="fs-17 font-semibold">Activity Rings</p>
        <div className="mt-12 flex items-center gap-18">
          <svg viewBox="0 0 110 110" className="size-110 shrink-0 -rotate-90" aria-hidden="true">
            {rings.map((g) => {
              const c = 2 * Math.PI * g.r;
              return (
                <g key={g.r}>
                  <circle cx="55" cy="55" r={g.r} fill="none" stroke={g.c} strokeOpacity="0.2" strokeWidth="11" />
                  <circle cx="55" cy="55" r={g.r} fill="none" stroke={g.c} strokeWidth="11" strokeLinecap="round" strokeDasharray={c} strokeDashoffset={c * (1 - g.p)} />
                </g>
              );
            })}
          </svg>
          <div className="space-y-6 leading-none">
            {[["Move", "412/530", "KCAL", "#ff375f"], ["Exercise", "22/30", "MIN", "#9cf02c"], ["Stand", "9/12", "HRS", "#2fe3f8"]].map(([k, v, u, c]) => (
              <div key={k}>
                <p className="fs-13 font-medium text-white/80">{k}</p>
                <p className="mt-2 fs-20 font-semibold tracking-[-0.02em]" style={{ color: c }}>
                  {v}
                  <span className="ml-3 fs-12 font-semibold">{u}</span>
                </p>
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="mx-16 mt-10 grid grid-cols-2 gap-10">
        {[
          ["Step Count", "6,420", [40, 62, 35, 80, 55, 92, 70]],
          ["Distance", "4.8 KM", [30, 45, 60, 52, 75, 68, 88]],
        ].map(([k, v, bars]) => (
          <div key={k as string} className="r-14 bg-[#1c1c1e] p-12">
            <p className="fs-13 font-semibold text-[#c387ff]">{k as string}</p>
            <p className="mt-4 fs-22 font-semibold tracking-[-0.02em]">{v as string}</p>
            <div className="mt-8 flex h-28 items-end gap-3">
              {(bars as number[]).map((h, i) => (
                <span key={i} className="flex-1 rounded-[1px] bg-[#c387ff]" style={{ height: `${h}%`, opacity: i === 6 ? 1 : 0.45 }} />
              ))}
            </div>
          </div>
        ))}
      </div>
      <div className="mx-16 mt-16 flex items-baseline justify-between">
        <p className="fs-22 font-bold tracking-[-0.02em]">Workouts</p>
        <p className="fs-15 text-[#9cf02c]">Show More</p>
      </div>
      <div className="mx-16 mt-8 flex items-center gap-12 r-14 bg-[#1c1c1e] p-10">
        <span className="size-56 overflow-hidden r-10">
          <Photo id={appPhotos.workout} w={112} h={112} />
        </span>
        <div className="flex-1 leading-tight">
          <p className="fs-16 font-semibold">Morning HIIT</p>
          <p className="mt-2 fs-13 text-white/55">32 min · 286 kcal · Yaba</p>
        </div>
        <ChevronRight className="size-18 text-white/35" />
      </div>
      <TabBar dark accent="#9cf02c" tabs={[[Activity, "Summary"], [Dumbbell, "Fitness+"], [Users, "Sharing"]]} />
    </div>
  );
}

/* ---------------- Pot & Pepper: recipes ---------------- */

const FOOD = "#e4572e";

function FoodApp() {
  const items = [
    ["Long grain parboiled rice", "4 cups"],
    ["Tatashe and tomato blend", "3 cups"],
    ["Scotch bonnet", "2"],
    ["Chicken stock", "2 cups"],
    ["Thyme, curry, bay leaf", "1 tsp each"],
  ];
  return (
    <div className="absolute inset-0 bg-white">
      <div className="absolute inset-x-0 top-0 h-360">
        <Photo id={appPhotos.jollof} w={786} h={720} />
        <div className="absolute inset-x-0 top-0 h-110 bg-gradient-to-b from-black/45 to-transparent" />
      </div>
      <StatusBar light />
      <div className="relative flex justify-between px-16 pt-4">
        <span className="grid size-36 place-items-center rounded-full bg-white/85 backdrop-blur"><ChevronLeft className="size-20" /></span>
        <div className="flex gap-8">
          <span className="grid size-36 place-items-center rounded-full bg-white/85 backdrop-blur"><Heart className="size-18" /></span>
          <span className="grid size-36 place-items-center rounded-full bg-white/85 backdrop-blur"><Share className="size-18" /></span>
        </div>
      </div>
      <div className="absolute inset-x-0 bottom-0 top-310 r-24 bg-white px-20 pt-20">
        <div className="flex items-center gap-8 fs-13 font-semibold">
          <span className="rounded-full px-10 py-3" style={{ background: "#fdeee8", color: FOOD }}>Party favourite</span>
          <span className="inline-flex items-center gap-3 text-black/55">
            <Star className="size-13 fill-[#f5a623] text-[#f5a623]" /> 4.9 · 2.1k
          </span>
        </div>
        <p className="mt-10 fs-28 font-bold leading-[1.1] tracking-[-0.03em]">Smoky Party Jollof</p>
        <div className="mt-10 flex items-center gap-8">
          <Avatar id="1687137113677-f2a9a6c79fab" size={24} />
          <p className="flex-1 fs-14 text-black/60">by Chef Funmi</p>
          <span className="rounded-full border border-black/15 px-12 py-4 fs-13 font-semibold">Follow</span>
        </div>
        <div className="mt-14 grid grid-cols-3 divide-x divide-black/10 r-12 bg-[#f6f5f3] py-10 text-center">
          {[[Clock, "45 min"], [Users, "Serves 6"], [Flame, "Medium"]].map(([Icon, l]) => {
            const I = Icon as LucideIcon;
            return (
              <div key={l as string} className="flex flex-col items-center gap-3">
                <I className="size-16 text-black/55" />
                <span className="fs-13 font-medium">{l as string}</span>
              </div>
            );
          })}
        </div>
        <div className="mt-14 grid h-36 grid-cols-2 r-10 bg-black/5 p-2 fs-14 font-semibold">
          <span className="grid place-items-center r-8 bg-white shadow-sm">Ingredients</span>
          <span className="grid place-items-center text-black/50">Method</span>
        </div>
        <ul className="mt-6 divide-y divide-black/[0.07]">
          {items.map(([n, q]) => (
            <li key={n} className="flex justify-between py-11 fs-15">
              <span>{n}</span>
              <span className="text-black/45">{q}</span>
            </li>
          ))}
        </ul>
      </div>
      <TabBar accent={FOOD} tabs={[[House, "Discover"], [Search, "Search"], [Bookmark, "Saved"], [Users, "Profile"]]} />
    </div>
  );
}

/* ---------------- Exam Ready: practice ---------------- */

const EDU = "#16a34a";

function ExamApp() {
  return (
    <div className="absolute inset-0 bg-white">
      <StatusBar />
      <div className="flex items-center gap-14 px-18 pt-6">
        <X className="size-22 text-black/40" />
        <span className="h-10 flex-1 overflow-hidden rounded-full bg-black/[0.07]">
          <span className="block h-full w-[30%] rounded-full" style={{ background: EDU }} />
        </span>
        <span className="inline-flex items-center gap-3 fs-15 font-bold text-[#f97316]">
          <Flame className="size-18 fill-[#f97316]" /> 7
        </span>
      </div>
      <div className="mt-22 flex items-center justify-between px-20">
        <span className="rounded-full bg-[#eaf7ee] px-10 py-4 fs-12 font-semibold" style={{ color: EDU }}>UTME · Chemistry</span>
        <span className="fs-13 font-medium text-black/45">Question 12 of 40</span>
      </div>
      <p className="mt-14 px-20 fs-24 font-bold leading-[1.2] tracking-[-0.02em]">
        Which gas is given off when zinc reacts with dilute hydrochloric acid?
      </p>
      <div className="mt-22 space-y-10 px-20">
        {["Oxygen", "Hydrogen", "Chlorine", "Carbon dioxide"].map((o, i) => {
          const on = i === 1;
          return (
            <div
              key={o}
              className="flex h-56 items-center gap-12 r-14 border-2 px-14 fs-17 font-medium"
              style={on ? { borderColor: EDU, background: "#eaf7ee" } : { borderColor: "rgb(0 0 0 / 0.09)" }}
            >
              <span
                className="grid size-28 place-items-center r-8 border-2 fs-13 font-bold"
                style={on ? { background: EDU, borderColor: EDU, color: "white" } : { borderColor: "rgb(0 0 0 / 0.12)", color: "rgb(0 0 0 / 0.45)" }}
              >
                {on ? <Check className="size-15" strokeWidth={3} /> : "ABCD"[i]}
              </span>
              {o}
            </div>
          );
        })}
      </div>
      <div className="absolute inset-x-20 bottom-38">
        <span className="grid h-52 place-items-center r-14 fs-17 font-bold text-white shadow-[0_4px_0_#15803d]" style={{ background: EDU }}>
          Check answer
        </span>
      </div>
    </div>
  );
}

/* ---------------- Groove: music ---------------- */

function MusicApp() {
  return (
    <div className="absolute inset-0 bg-[linear-gradient(180deg,#6b4430_0%,#3b261b_45%,#121212_100%)] text-white">
      <StatusBar light />
      <div className="flex items-center justify-between px-20 pt-4">
        <ChevronDown className="size-24" />
        <div className="text-center leading-tight">
          <p className="fs-11 font-medium uppercase tracking-[0.08em] text-white/70">Playing from playlist</p>
          <p className="fs-13 font-bold">Lagos Nights</p>
        </div>
        <Ellipsis className="size-22" />
      </div>
      <div className="mx-24 mt-28 aspect-square overflow-hidden r-8 shadow-[0_24px_48px_-16px_rgb(0_0_0/0.7)]">
        <Photo id={appPhotos.album} w={690} h={690} />
      </div>
      <div className="mt-28 flex items-center justify-between px-24">
        <div className="min-w-0 leading-tight">
          <p className="fs-22 font-bold tracking-[-0.02em]">Island Time</p>
          <p className="mt-2 fs-16 text-white/65">Ayo &amp; The Tide</p>
        </div>
        <span className="grid size-26 place-items-center rounded-full bg-[#1fd760]">
          <Check className="size-15 text-black" strokeWidth={3} />
        </span>
      </div>
      <div className="mt-20 px-24">
        <div className="relative h-4 rounded-full bg-white/25">
          <div className="h-full w-[42%] rounded-full bg-white" />
          <span className="absolute left-[42%] top-1/2 size-12 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white" />
        </div>
        <div className="mt-8 flex justify-between fs-11 text-white/60">
          <span>1:42</span>
          <span>-2:16</span>
        </div>
      </div>
      <div className="mt-10 flex items-center justify-between px-24">
        <Shuffle className="size-22 text-[#1fd760]" />
        <SkipBack className="size-30 fill-white" />
        <span className="grid size-64 place-items-center rounded-full bg-white">
          <Pause className="size-26 fill-black text-black" />
        </span>
        <SkipForward className="size-30 fill-white" />
        <Repeat2 className="size-22 text-white/70" />
      </div>
      <div className="mt-20 flex items-center justify-between px-24 text-white/70">
        <Speaker className="size-18" />
        <div className="flex gap-22">
          <Share2 className="size-18" />
          <ListMusic className="size-18" />
        </div>
      </div>
    </div>
  );
}

/* ---------------- Lookbook: style board ---------------- */

function FashionApp() {
  const [a, b, c, d] = appPhotos.looks;
  const Pin = ({ id, h, caption }: { id: string; h: number; caption?: string }) => (
    <div>
      <div className="overflow-hidden r-16" style={{ height: `calc(var(--spacing) * ${h})` }}>
        <Photo id={id} w={360} h={h * 2} />
      </div>
      {caption && (
        <div className="flex items-center justify-between px-4 pb-6 pt-6">
          <p className="truncate fs-12 font-medium text-black/75">{caption}</p>
          <Ellipsis className="size-15 shrink-0 text-black/50" />
        </div>
      )}
    </div>
  );
  return (
    <div className="absolute inset-0 bg-white">
      <StatusBar />
      <div className="mx-16 flex h-40 items-center gap-8 rounded-full bg-black/[0.055] px-14">
        <Search className="size-17 text-black/45" />
        <span className="flex-1 truncate fs-15 text-black/40">Search ankara, owambe, aso-ebi</span>
        <Camera className="size-18 text-black/50" />
      </div>
      <div className="mt-12 flex gap-18 overflow-hidden px-18 fs-15 font-semibold">
        <span className="border-b-2 border-black pb-6">For you</span>
        {["Owambe", "Work", "Aso-ebi", "Street"].map((t) => (
          <span key={t} className="text-black/45">{t}</span>
        ))}
      </div>
      <div className="mt-10 grid grid-cols-2 gap-8 px-8">
        <div>
          <Pin id={a} h={250} caption="Red wrapper, gold accents" />
          <Pin id={d} h={210} />
        </div>
        <div>
          <Pin id={b} h={190} caption="Sunday best" />
          <Pin id={c} h={270} caption="Matching two-piece" />
        </div>
      </div>
      <TabBar labels={false} accent="#111" tabs={[[House, "Home"], [Search, "Search"], [Plus, "Create"], [MessageCircle, "Inbox"], [Users, "Profile"]]} />
    </div>
  );
}

const screens: Record<NicheId, () => React.JSX.Element> = {
  faith: BibleApp,
  fitness: FitnessApp,
  food: FoodApp,
  education: ExamApp,
  music: MusicApp,
  fashion: FashionApp,
};

/** Where a banner sits in each app, in points from the bottom. */
const bannerSlot: Record<NicheId, { compact: boolean; bottom: number }> = {
  faith: { compact: false, bottom: 93 },
  fitness: { compact: true, bottom: 93 },
  food: { compact: false, bottom: 93 },
  education: { compact: true, bottom: 104 },
  music: { compact: true, bottom: 30 },
  fashion: { compact: false, bottom: 93 },
};

const darkApps: NicheId[] = ["fitness", "music"];

/** Lock screen, where a notification ad really lands. */
export function LockScreen({ niche }: { niche: Niche }) {
  return (
    <div className="absolute inset-0 overflow-hidden bg-[#2a2622] text-white">
      <div className="absolute inset-0">
        <Photo id={appPhotos.sunrise} w={786} h={1704} />
      </div>
      <div className="absolute inset-0 bg-gradient-to-b from-black/15 via-black/10 to-black/55" />
      <div className="relative flex h-54 items-center justify-between pl-34 pr-30 pt-6 text-white">
        <span className="fs-15 font-semibold">MTN NG</span>
        <span aria-hidden className="absolute left-1/2 top-11 h-37 w-126 -translate-x-1/2 rounded-full bg-black" />
        <span className="fs-13 font-semibold">82%</span>
      </div>
      <div className="relative mt-6 flex justify-center">
        <Lock className="size-17" strokeWidth={2.4} />
      </div>
      <div className="relative mt-8 text-center">
        <p className="fs-21 font-semibold text-white/90">Saturday 14 June</p>
        <p className="mt-[-2%] fs-96 font-semibold leading-none tracking-[-0.04em]">9:41</p>
      </div>
      <div className="relative mt-10 flex justify-center gap-10">
        {[["31°", "Lagos"], ["82%", "Battery"], ["6:47", "Sunset"]].map(([v, l]) => (
          <div key={l} className="grid size-56 place-items-center rounded-full bg-white/15 text-center leading-none backdrop-blur-md">
            <div>
              <p className="fs-15 font-semibold">{v}</p>
              <p className="mt-2 fs-9 font-medium text-white/70">{l}</p>
            </div>
          </div>
        ))}
      </div>
      <div className="absolute inset-x-8 bottom-128 space-y-8">
        <NotificationAd niche={niche} />
        <div className="mx-10 flex items-center gap-10 r-20 bg-[rgb(245_245_247/0.55)] px-14 py-10 text-black/70 backdrop-blur-2xl">
          <span className="grid size-26 place-items-center r-7 bg-[#34c759]">
            <MessageCircle className="size-15 fill-white text-white" />
          </span>
          <p className="flex-1 truncate fs-13"><span className="font-semibold text-black/80">Mum</span> · Call me when you’re free</p>
          <p className="fs-12 text-black/45">9:12</p>
        </div>
      </div>
      <div className="absolute inset-x-46 bottom-46 flex justify-between">
        {[Flashlight, Camera].map((I, i) => (
          <span key={i} className="grid size-50 place-items-center rounded-full bg-black/35 backdrop-blur-md">
            <I className="size-22" />
          </span>
        ))}
      </div>
      <HomeIndicator light />
    </div>
  );
}

/** A host app with the ad in its proper slot. */
export function AppScreen({
  niche,
  format,
  host,
  showAd = true,
}: {
  /** The ad being shown. */
  niche: Niche;
  format: Format;
  /** The app it's shown in. Defaults to the ad's own niche. */
  host?: NicheId;
  showAd?: boolean;
}) {
  const id = host ?? niche.id;
  const Screen = screens[id];
  const slot = bannerSlot[id];
  return (
    <div className="absolute inset-0 overflow-hidden">
      <Screen />
      {showAd && format === "banner" && (
        <div className="absolute inset-x-12 z-10" style={{ bottom: `calc(var(--spacing) * ${slot.bottom})` }}>
          {slot.compact ? <CompactBannerAd niche={niche} /> : <BannerAd niche={niche} />}
        </div>
      )}
      {showAd && format === "notification" && (
        <div className="absolute inset-x-8 top-56 z-30">
          <NotificationAd niche={niche} />
        </div>
      )}
      <HomeIndicator light={darkApps.includes(id)} />
    </div>
  );
}
