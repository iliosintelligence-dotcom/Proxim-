import { ChartColumn, Compass, Crosshair, Lightbulb, MapPin, Users } from "lucide-react";
import type { Metadata } from "next";
import { PageHero } from "@/components/site/PageHero";
import { Section } from "@/components/site/Frame";
import { AppIcon, type IconTone } from "@/components/site/AppIcon";
import type { LucideIcon } from "lucide-react";
import { FinalCta } from "@/components/site/FinalCta";
import { CitiesGlobe } from "@/components/about/CitiesGlobe";
import { PhotoBand } from "@/components/site/PhotoBand";
import { PhoneTicker } from "@/components/site/Tickers";
import { ProximNote } from "@/components/phone/Ads";
import { HIcon } from "@/components/site/HIcon";

export const metadata: Metadata = {
  title: "About: Proxim",
  description: "Proxim connects local businesses with the apps their customers already use. Built in Nigeria, for Nigeria.",
};

const principles: { icon: LucideIcon; tone: IconTone; title: string; body: string }[] = [
  {
    icon: Crosshair,
    tone: "blue",
    title: "Relevant, or not at all",
    body: "An ad that matches what someone is doing feels like a tip. One that doesn’t is noise. We only place the first kind.",
  },
  {
    icon: MapPin,
    tone: "orange",
    title: "Local by default",
    body: "Most businesses serve a few kilometres around them. Our targeting starts there, not at the whole country.",
  },
  {
    icon: Users,
    tone: "green",
    title: "Fair to both sides",
    body: "Advertisers pay only for real views. App owners keep most of every naira. Users see fewer, better ads.",
  },
  {
    icon: ChartColumn,
    tone: "purple",
    title: "Honest numbers",
    body: "Every view and tap is shown as it happens. If a campaign isn’t working, you’ll see it before you’ve spent the budget.",
  },
];

export default function AboutPage() {
  return (
    <>
      <PageHero
        label="About Proxim"
        icon={Users}
        title="Advertising should feel like a recommendation."
        lead="Proxim exists so the bakery down the road, the church across town and the tutor in your area can reach the people who’d actually want to hear from them."
        align="left"
        ticker={<PhoneTicker />}
      />

      <Section index="01" label="Why we started" icon={Lightbulb} pip="This is why I exist.">
        <div className="grid gap-12 lg:grid-cols-[1fr_1fr] lg:gap-20">
          <blockquote data-led className="t-section max-w-[16ch]">
            Small businesses were paying to be ignored.
          </blockquote>
          <div className="space-y-5 text-[1.06rem] leading-relaxed text-ink-2">
            <p data-reveal>
              A restaurant in Surulere runs an online ad. It’s shown to people in Kano, to people who don’t eat out, to
              people scrolling straight past. The restaurant pays for all of it.
            </p>
            <p data-reveal>
              Meanwhile, the apps people actually spend time in, the Bible app read every morning, the step counter
              checked after every walk, have audiences that local businesses would love to reach, and no simple way to
              earn from them.
            </p>
            <p data-reveal>
              Proxim joins the two. Businesses choose the kind of app, the area and the dates. App owners choose which
              ads they’ll show. People see offers that fit what they’re doing and where they are.
            </p>
          </div>
        </div>
      </Section>

      <Section index="02" label="What we believe" tone="stone" icon={Compass}>
        <h2 data-led className="t-section max-w-[18ch]">
          Four rules we <HIcon photo="1687422808328-11cf750a5051" /> don’t bend.
        </h2>
        <ul className="mt-14 grid gap-4 sm:grid-cols-2">
          {principles.map((p, i) => (
            <li key={p.title} data-reveal className="flex flex-col rounded-3xl border border-[var(--line)] bg-white p-7 sm:p-8">
              <div className="flex items-center justify-between">
                <AppIcon icon={p.icon} tone={p.tone} />
                <span className="t-label text-faint">0{i + 1}</span>
              </div>
              <h3 className="t-card mt-8">{p.title}</h3>
              <p className="mt-2 text-[0.96rem] leading-relaxed text-muted-foreground">{p.body}</p>
            </li>
          ))}
        </ul>
      </Section>

      <PhotoBand
        photo="1585540083814-ea6ee8af9e4f"
        label="Where we’re starting"
        icon={MapPin}
        title={
          <>
            Built in <HIcon photo="1665849863678-fc03a642a9aa" /> Nigeria, for Nigeria.
          </>
        }
        lead="Priced in naira, paid to Nigerian bank accounts, and designed around how people here actually use their phones. We’re starting in eight cities and growing from there."
        aside={<CitiesGlobe />}
      />

      <FinalCta
        statement="Advertising should feel like a recommendation"
        title="Two sides. One network."
        lead="Whether you’re a business that wants to be found or an app with an audience worth paying for, there’s a place for you on Proxim."
        primary={{ href: "/#launch", label: "Advertise with Proxim" }}
        secondary={{ href: "/publishers", label: "Earn with your app" }}
        cards={[
          <ProximNote key="a" title="Campaign live in Lekki" body="Grace Assembly is now showing in 3 faith apps nearby." />,
          <ProximNote key="b" title="New app joined" body="Exam Ready is now taking ads from tutors in Ibadan." time="1h" />,
          <ProximNote key="c" title="Payout sent" body="Groove was paid for October’s views." time="1d" />,
        ]}
      />
    </>
  );
}
