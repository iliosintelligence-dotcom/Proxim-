import { Layers } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { PageHero } from "@/components/site/PageHero";
import { TargetingSection } from "@/components/features/TargetingSection";
import { FormatSpecs } from "@/components/features/FormatSpecs";
import { ReportingSection } from "@/components/features/ReportingSection";
import { SafetySection } from "@/components/features/SafetySection";
import { FinalCta } from "@/components/site/FinalCta";
import { BuilderDemo } from "@/components/features/BuilderDemo";
import { NotificationAd } from "@/components/phone/Ads";
import { nicheById } from "@/lib/data";

export const metadata: Metadata = {
  title: "Features: Proxim",
  description: "Niche matching, proximity targeting, banner and notification ads, live reporting and brand safety.",
};

const jumps = [
  ["Targeting", "#targeting"],
  ["Formats", "#formats"],
  ["Reporting", "#reporting"],
  ["Trust and safety", "#safety"],
];

export default function FeaturesPage() {
  return (
    <>
      <PageHero
        label="Features"
        icon={Layers}
        title="Everything it takes to reach the right people nearby."
        lead="Proxim decides three things for every ad: which apps it runs in, how far from you, and for how long. Here’s how each part works."
      >
        <div className="flex flex-wrap justify-center gap-2">
          {jumps.map(([l, h]) => (
            <Link
              key={h}
              href={h}
              className="rounded-full border border-[var(--line-strong)] bg-white px-4 py-2 text-[0.88rem] font-medium text-ink-2 transition-colors hover:border-ink/30 hover:text-ink"
            >
              {l}
            </Link>
          ))}
        </div>
        <BuilderDemo />
      </PageHero>
      <TargetingSection />
      <FormatSpecs />
      <ReportingSection />
      <SafetySection />
      <FinalCta
        statement="Right app. {0} Right place. {1} Right time."
        photos={["1649888216899-047093431441", "1687422808191-93810cd07ab0"]}
        title="See it with your own business."
        lead="Build a campaign in two minutes and see exactly who it would reach and what it would cost. No card needed."
        primary={{ href: "/#launch", label: "Build a campaign" }}
        secondary={{ href: "/pricing", label: "See pricing" }}
        cards={(["education", "music", "fashion"] as const).map((id) => (
          <NotificationAd key={id} niche={nicheById(id)} />
        ))}
      />
    </>
  );
}
