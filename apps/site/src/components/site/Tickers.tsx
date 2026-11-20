import { ArrowRight, Sparkle } from "lucide-react";
import { niches, nicheById, type Format, type NicheId } from "@/lib/data";
import { Phone } from "@/components/phone/Phone";
import { AppScreen, LockScreen } from "@/components/phone/AppScreens";
import { Photo } from "@/components/phone/Photo";
import { Marquee } from "@/components/motion/Marquee";

/* Sample activity for the tickers. Illustrative, not live data. */
const deliveries = [
  { id: "faith", phones: 312, area: "Lekki", ago: "2s" },
  { id: "food", phones: 186, area: "Surulere", ago: "5s" },
  { id: "fitness", phones: 94, area: "Yaba", ago: "9s" },
  { id: "music", phones: 421, area: "Wuse 2", ago: "14s" },
  { id: "education", phones: 238, area: "Bodija", ago: "21s" },
  { id: "fashion", phones: 157, area: "GRA", ago: "30s" },
] as const;

/** Glass pills of ads being delivered, for use over photography. */
export function DeliveryTicker() {
  return (
    <Marquee seconds={46} className="[mask-image:linear-gradient(to_right,transparent,black_6%,black_94%,transparent)]">
      {deliveries.map((d) => {
        const n = niches.find((x) => x.id === d.id)!;
        return (
          <span
            key={d.id}
            className="flex items-center gap-3 whitespace-nowrap rounded-full border border-white/15 bg-white/10 py-1.5 pl-1.5 pr-4 text-[0.88rem] text-white backdrop-blur-md"
          >
            <span className="block size-8 overflow-hidden rounded-full ring-1 ring-white/30">
              <Photo id={n.ad.photo} w={64} h={64} />
            </span>
            <span className="font-medium">{n.ad.brand}</span>
            <ArrowRight className="size-3.5 text-white/50" />
            <span className="text-white/80">
              {d.phones} phones in {d.area}
            </span>
            <span className="text-white/40">{d.ago} ago</span>
          </span>
        );
      })}
    </Marquee>
  );
}

const earnings = [
  { app: "Daily Word", photo: "1665849863678-fc03a642a9aa", amount: "₦18,420", when: "this week" },
  { app: "Stride", photo: "1649888216899-047093431441", amount: "₦9,860", when: "this week" },
  { app: "Pot & Pepper", photo: "1666190092689-e3968aa0c32c", amount: "₦24,300", when: "this week" },
  { app: "Exam Ready", photo: "1529390079861-591de354faf5", amount: "₦31,150", when: "this week" },
  { app: "Groove", photo: "1643904524951-2a3a58856745", amount: "₦12,940", when: "this week" },
  { app: "Lookbook", photo: "1625646741211-711bdd65c570", amount: "₦7,280", when: "this week" },
];

/** What apps on the network are earning. */
export function EarningsTicker() {
  return (
    <Marquee seconds={44} reverse className="[mask-image:linear-gradient(to_right,transparent,black_6%,black_94%,transparent)]">
      {earnings.map((e) => (
        <span
          key={e.app}
          className="flex items-center gap-3 whitespace-nowrap rounded-full border border-white/15 bg-white/10 py-1.5 pl-1.5 pr-4 text-[0.88rem] text-white backdrop-blur-md"
        >
          <span className="block size-8 overflow-hidden rounded-[9px] ring-1 ring-white/30">
            <Photo id={e.photo} w={64} h={64} />
          </span>
          <span className="font-medium">{e.app}</span>
          <span className="text-white/60">earned</span>
          <span className="font-display text-[1rem] tracking-[-0.02em] text-ad-lime">{e.amount}</span>
          <span className="text-white/40">{e.when}</span>
        </span>
      ))}
    </Marquee>
  );
}

/** Big pixel-type brand line that runs above the footer on every page. */
export function BrandTicker() {
  const lines = ["Put your ad where it belongs", "Reach people nearby", "Get paid for your attention", "Pay only for views"];
  return (
    <div className="border-t bg-night py-8 text-white sm:py-10" style={{ borderColor: "var(--line-night)" }}>
      <Marquee seconds={60} gap="gap-10">
        {lines.map((l) => (
          <span key={l} className="flex items-center gap-10 whitespace-nowrap font-display text-[clamp(2.4rem,6vw,4.75rem)] leading-none tracking-[-0.04em]">
            {l}
            <Sparkle className="size-[0.5em] fill-blue text-blue" strokeWidth={1.5} />
          </span>
        ))}
      </Marquee>
    </div>
  );
}

const businesses = [
  { label: "Kitchens", place: "Surulere", photo: "1698827623494-c4e1196ba0ca" },
  { label: "Tailors", place: "GRA", photo: "1687422809069-0fa3546b8471" },
  { label: "Corner shops", place: "Ajah", photo: "1687422808311-a776f467a468" },
  { label: "Salons", place: "Ikeja", photo: "1702236240794-58dc4c6895e5" },
  { label: "Butchers", place: "Lekki", photo: "1687422808248-f807f4ea2a2e" },
  { label: "Churches", place: "Lekki", photo: "1609532587710-b00027c0594b" },
  { label: "Pharmacies", place: "Wuse", photo: "1576091358783-a212ec293ff3" },
  { label: "Barbers", place: "Yaba", photo: "1687422808328-11cf750a5051" },
  { label: "Market traders", place: "Bodija", photo: "1575303093127-18b3c4ef8c41" },
  { label: "Gyms", place: "Yaba", photo: "1597347343908-2937e7dcc560" },
  { label: "Bakeries", place: "Ikeja", photo: "1709837167686-a2e33aad1bf0" },
  { label: "Tutors", place: "Bodija", photo: "1744809482817-9a9d4fc280af" },
];

/** Edge-to-edge belt of the kinds of businesses Proxim is for. */
export function BusinessTicker() {
  return (
    <Marquee seconds={55} gap="gap-3" className="w-full py-1">
      {businesses.map((b) => (
        <span
          key={b.label}
          className="flex items-center gap-3 whitespace-nowrap rounded-full border border-[var(--line)] bg-white py-1.5 pl-1.5 pr-5 shadow-[0_1px_2px_rgb(0_0_0/0.04)]"
        >
          <span className="block size-11 overflow-hidden rounded-full">
            <Photo id={b.photo} w={88} h={88} />
          </span>
          <span className="leading-tight">
            <span className="block font-display text-[1.15rem] tracking-[-0.02em] text-ink">{b.label}</span>
            <span className="block text-[0.78rem] text-muted-foreground">{b.place}</span>
          </span>
        </span>
      ))}
    </Marquee>
  );
}

const screens: { niche: NicheId; format: Format; lock?: boolean; caption: string }[] = [
  { niche: "faith", format: "banner", caption: "Banner in a Bible app" },
  { niche: "fitness", format: "notification", caption: "Notification in a step counter" },
  { niche: "food", format: "banner", caption: "Banner in a recipe app" },
  { niche: "food", format: "notification", lock: true, caption: "On the lock screen" },
  { niche: "music", format: "notification", caption: "Notification in a music app" },
  { niche: "fashion", format: "banner", caption: "Banner in a style app" },
  { niche: "education", format: "banner", caption: "Banner in an exam app" },
  { niche: "faith", format: "notification", lock: true, caption: "On the lock screen" },
];

/**
 * Belt of phones, each showing an ad as a banner or notification. Kept
 * small on phones so it reads as a strip, not a wall of screens.
 */
export function PhoneTicker() {
  return (
    <div data-ad-manual className="ads-visible">
      <Marquee seconds={70} gap="gap-4 sm:gap-6" className="w-full py-6">
        {screens.map((s, i) => {
          const n = nicheById(s.niche);
          return (
            <figure key={i} className="w-[7.5rem] shrink-0 sm:w-[10rem] lg:w-[12rem]">
              <Phone label={`${n.ad.brand} ${s.format} ad`}>
                {s.lock ? <LockScreen niche={n} /> : <AppScreen niche={n} format={s.format} />}
              </Phone>
              <figcaption className="mt-2.5 text-center text-[0.7rem] leading-snug font-medium text-muted-foreground sm:mt-3 sm:text-[0.8rem]">
                {s.caption}
              </figcaption>
            </figure>
          );
        })}
      </Marquee>
    </div>
  );
}
