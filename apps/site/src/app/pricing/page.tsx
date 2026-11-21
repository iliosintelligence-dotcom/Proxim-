import { AppWindow, Banknote, CircleCheck, MessageCircleQuestion, Receipt } from "lucide-react";
import type { Metadata } from "next";
import { PageHero } from "@/components/site/PageHero";
import { Section } from "@/components/site/Frame";
import { Button } from "@/components/site/Button";
import { Accordion } from "@/components/site/Accordion";
import { AppIcon } from "@/components/site/AppIcon";
import { FinalCta } from "@/components/site/FinalCta";
import { CampaignProvider } from "@/components/advertisers/CampaignContext";
import { LaunchSection } from "@/components/advertisers/LaunchSection";
import { NotificationAd } from "@/components/phone/Ads";
import { nicheById, plans, pricingFaqs, publisherShare } from "@/lib/data";
import { HIcon } from "@/components/site/HIcon";

export const metadata: Metadata = {
  title: "Pricing: Proxim",
  description: "Pay only for views that are seen. Banner and notification ads from ₦6,000, priced in naira with no contract.",
};

export default function PricingPage() {
  return (
    <>
      <PageHero
        label="Pricing"
        icon={Receipt}
        title="Simple pricing. Pay only for what’s seen."
        lead="No contracts, no setup fee, no dollar card. You pay in naira for views that actually reach a screen, and you see the full price before you launch."
      />

      <section className="bg-paper">
        <div className="mx-auto w-full max-w-[1240px] px-[var(--gutter)] pb-[var(--section-y)] lg:border-x" style={{ borderColor: "var(--line)" }}>
          <div className="grid gap-4 lg:grid-cols-3">
            {plans.map((p) => (
              <article
                key={p.id}
                data-reveal
                className={`relative flex flex-col rounded-3xl border p-7 ${
                  p.featured
                    ? "border-ink bg-night text-white shadow-[0_30px_60px_-30px_rgb(28_25_23/0.6)]"
                    : "border-[var(--line)] bg-white"
                }`}
              >
                {p.featured && (
                  <span className="t-label absolute right-6 top-7 rounded-full bg-blue px-2.5 py-1 text-white">Most popular</span>
                )}
                <h2 className="t-card">{p.name}</h2>
                <p className={`mt-2 min-h-[3rem] text-[0.92rem] leading-relaxed ${p.featured ? "text-white/55" : "text-muted-foreground"}`}>{p.pitch}</p>
                <p className="mt-6 font-display text-[2.6rem] leading-none tracking-[-0.04em]">{p.price}</p>
                <p className={`mt-1.5 text-[0.84rem] ${p.featured ? "text-white/45" : "text-faint"}`}>{p.per}</p>
                <ul className={`mt-7 space-y-3 border-t pt-6 text-[0.92rem] ${p.featured ? "border-white/10 text-white/80" : "border-[var(--line)] text-ink-2"}`}>
                  {p.features.map((f) => (
                    <li key={f} className="flex items-start gap-3">
                      <CircleCheck className={`mt-[0.15rem] size-4 shrink-0 ${p.featured ? "text-ad-lime" : "text-blue"}`} strokeWidth={2.2} />
                      {f}
                    </li>
                  ))}
                </ul>
                <div className="mt-auto pt-8">
                  <Button
                    href={p.id === "agency" ? "/contact" : "/#launch"}
                    variant={p.featured ? "night" : "secondary"}
                    arrow
                    className="w-full"
                  >
                    {p.cta}
                  </Button>
                </div>
              </article>
            ))}
          </div>
          <p data-reveal className="mt-5 text-center text-[0.8rem] text-faint">Sample plans and rates. Final pricing may differ.</p>
        </div>
      </section>

      <CampaignProvider>
        <LaunchSection
          id="calculator"
          index="01"
          label="Calculator"
          title={
            <>
              Price your <HIcon photo="1709837167686-a2e33aad1bf0" /> campaign in seconds.
            </>
          }
          withPickers
        />
      </CampaignProvider>

      <Section index="02" label="App owners" tone="stone" icon={AppWindow}>
        <div className="grid items-center gap-10 rounded-3xl border border-[var(--line)] bg-white p-7 sm:p-10 lg:grid-cols-[auto_1fr_auto]">
          <AppIcon icon={Banknote} tone="yellow" size="lg" className="!size-16" />
          <div>
            <h2 data-led className="t-card !text-[clamp(1.5rem,2.6vw,2rem)]">
              Own an app? It’s free, and you keep {Math.round(publisherShare * 100)}%.
            </h2>
            <p data-reveal className="mt-2 max-w-[40rem] text-[0.98rem] leading-relaxed text-muted-foreground">
              App owners never pay to join. You keep {Math.round(publisherShare * 100)}% of what advertisers spend in your
              app, paid monthly in naira.
            </p>
          </div>
          <Button href="/publishers" variant="secondary" arrow className="justify-self-start lg:justify-self-auto">
            Earn with Proxim
          </Button>
        </div>
      </Section>

      <Section index="03" label="Questions" icon={MessageCircleQuestion} pip="Still unsure? Ask the team. They reply fast.">
        <div className="grid gap-12 lg:grid-cols-[0.8fr_1.2fr] lg:gap-20">
          <div>
            <h2 data-led className="t-section max-w-[12ch]">
              Questions, <HIcon photo="1687422808311-a776f467a468" /> answered.
            </h2>
            <p data-reveal className="t-lead mt-6 max-w-[26rem]">
              Can’t find what you need? Our team replies within one working day.
            </p>
            <div data-reveal className="mt-8">
              <Button href="/contact" variant="secondary" arrow>
                Contact us
              </Button>
            </div>
          </div>
          <Accordion items={pricingFaqs} />
        </div>
      </Section>

      <FinalCta
        statement="Pay for {0} views, not for hope"
        photos={["1709837167686-a2e33aad1bf0"]}
        title="Start with one campaign."
        lead="Pick your niche, draw your radius and launch. If it doesn’t work for you, stop any time. You only pay for views delivered."
        primary={{ href: "/#launch", label: "Launch a campaign" }}
        secondary={{ href: "/contact", label: "Talk to sales" }}
        cards={(["food", "faith", "fitness"] as const).map((id) => (
          <NotificationAd key={id} niche={nicheById(id)} />
        ))}
      />
    </>
  );
}
