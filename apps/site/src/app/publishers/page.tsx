import type { Metadata } from "next";
import { PubHero } from "@/components/publishers/PubHero";
import { Estimator } from "@/components/publishers/Estimator";
import { ControlSection } from "@/components/publishers/ControlSection";
import { IntegrationSection } from "@/components/publishers/IntegrationSection";
import { PayoutSection } from "@/components/publishers/PayoutSection";
import { Ticker } from "@/components/site/Ticker";
import { FinalCta } from "@/components/site/FinalCta";
import { ProximNote } from "@/components/phone/Ads";
import { Store } from "lucide-react";
import { PhotoBand } from "@/components/site/PhotoBand";
import { EarningsTicker } from "@/components/site/Tickers";

export const metadata: Metadata = {
  title: "Proxim for app owners: Earn from local ads that fit your app",
  description:
    "Add Proxim to your app and earn from niche, local ads. You choose the categories and frequency. Paid monthly in naira.",
};

export default function PublishersPage() {
  return (
    <>
      <PubHero />
      <Ticker
        label="Built for apps like"
      />
      <Estimator />
      <PhotoBand
        photo="1687422809654-579d81c29d32"
        label="The network"
        pip="Other apps are already earning with me."
        icon={Store}
        title="Apps like yours are already earning."
        lead="Scripture apps, step counters, recipe apps and exam trainers across Nigeria turn their daily users into monthly income with Proxim."
      >
        <div data-reveal className="mt-12 -mx-[var(--gutter)]">
          <EarningsTicker />
        </div>
        <p className="mt-4 text-[0.75rem] text-white/40">Sample figures from fictional example apps.</p>
      </PhotoBand>
      <ControlSection />
      <IntegrationSection />
      <PayoutSection />
      <FinalCta
        id="start"
        statement="Your users {0} are worth more than you {1} think"
        photos={["1620829813573-7c9e1877706f", "1643904524951-2a3a58856745"]}
        title="Your users are already here. Get paid for it."
        lead="Join free, set your rules, and start earning from businesses near your users. Your first payout could land next month."
        primary={{ href: "#", label: "Start earning" }}
        secondary={{ href: "#integrate", label: "Read the docs" }}
        cards={[
          <ProximNote key="a" title="You earned ₦18,420 this week" body="Up 22% on last week. Banners are doing the heavy lifting." />,
          <ProximNote key="b" title="New advertiser wants in" body="Grace Assembly, Lekki would like to show in Daily Word." time="2h" />,
          <ProximNote key="c" title="Payout sent" body="₦482,300 is on its way to your bank account ending 4821." time="1d" />,
          <ProximNote key="d" title="Your app is live on Proxim" body="Ads will start filling within the hour." time="30d" />,
        ]}
      />
    </>
  );
}
