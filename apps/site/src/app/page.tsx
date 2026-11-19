import { Users } from "lucide-react";
import { HIcon } from "@/components/site/HIcon";
import { PhotoBand } from "@/components/site/PhotoBand";
import { DeliveryTicker } from "@/components/site/Tickers";
import { CampaignProvider } from "@/components/advertisers/CampaignContext";
import { Hero } from "@/components/advertisers/Hero";
import { NicheSection } from "@/components/advertisers/NicheSection";
import { FormatSection } from "@/components/advertisers/FormatSection";
import { CardStack } from "@/components/advertisers/CardStack";
import { ProximitySection } from "@/components/advertisers/ProximitySection";
import { LaunchSection } from "@/components/advertisers/LaunchSection";
import { Ticker } from "@/components/site/Ticker";
import { FinalCta } from "@/components/site/FinalCta";
import { NotificationAd } from "@/components/phone/Ads";
import { nicheById } from "@/lib/data";

export default function AdvertisersPage() {
  return (
    <CampaignProvider>
      <Hero />
      <Ticker />
      <NicheSection />
      <PhotoBand
        photo="1687422808248-f807f4ea2a2e"
        label="Who it’s for"
        pip="Shops like these get found by people nearby."
        icon={Users}
        title="Built for the businesses that keep Nigeria running."
        lead="Kitchens, tailors, salons, pharmacies, churches and corner shops. If your customers live nearby, Proxim puts you on their phones while they’re in the apps they love."
      >
        <div data-reveal className="mt-10 grid max-w-[34rem] grid-cols-3 gap-3">
          {[["8", "cities live"], ["6", "app niches"], ["2", "ad formats"]].map(([v, l]) => (
            <div key={l} className="rounded-2xl border border-white/15 bg-white/10 p-4 backdrop-blur-md">
              <p className="font-display text-[2rem] leading-none tracking-[-0.04em]">{v}</p>
              <p className="mt-1.5 text-[0.82rem] text-white/65">{l}</p>
            </div>
          ))}
        </div>
        <div data-reveal className="mt-12 -mx-[var(--gutter)]">
          <DeliveryTicker />
        </div>
      </PhotoBand>
      <CardStack />
      <FormatSection />
      <ProximitySection />
      <LaunchSection
        title={
          <>
            Pay for <HIcon photo="1687422808565-929533931584" /> views. Not for hope.
          </>
        }
      />
      <FinalCta
        title="Your next customer is already scrolling."
        lead="They’re in a Bible app, a step counter, a recipe app, a few streets from you. Put your business on their screen today."
        primary={{ href: "#launch", label: "Launch a campaign" }}
        secondary={{ href: "#", label: "Talk to us" }}
        pip="Ready? I’ll deliver your first ad today."
        cards={(["faith", "fitness", "food", "music"] as const).map((id) => (
          <NotificationAd key={id} niche={nicheById(id)} />
        ))}
      />
    </CampaignProvider>
  );
}
