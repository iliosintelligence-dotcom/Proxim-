import { Banknote } from "lucide-react";
import { Section } from "@/components/site/Frame";
import { HIcon } from "@/components/site/HIcon";

const steps = [
  {
    n: "01",
    title: "Add Proxim",
    body: "Install the SDK, place your ad slots and set your rules. Most teams finish the same day.",
  },
  {
    n: "02",
    title: "Ads start filling",
    body: "Local businesses that match your app’s niche bid for your slots. You approve, they show.",
  },
  {
    n: "03",
    title: "Get paid in naira",
    body: "Earnings land in your Nigerian bank account every month. No minimum contract, no dollar card.",
  },
];

export function PayoutSection() {
  return (
    <Section id="payouts" index="04" label="Payouts" icon={Banknote} pip="Paid in naira, straight to your bank.">
      <div className="grid gap-14 lg:grid-cols-[1fr_minmax(0,27rem)] lg:gap-20">
        <div>
          <h2 data-led className="t-section max-w-[16ch]">
            Paid in <HIcon photo="1575303093127-18b3c4ef8c41" /> naira, every month.
          </h2>
          <p data-reveal className="t-lead mt-6 max-w-[33rem]">
            Track every view and every naira in your dashboard as it happens. At the end of the month it goes straight to
            your bank.
          </p>
        </div>

        <div data-reveal>
          <div className="overflow-hidden rounded-3xl border border-[var(--line)] bg-white shadow-[0_30px_60px_-30px_rgb(28_25_23/0.35)]">
            <div className="flex items-center justify-between px-6 pb-4 pt-6">
              <p className="font-display text-[1.2rem] tracking-[-0.02em]">October payout</p>
              <span className="t-label inline-flex items-center gap-1.5 rounded-full bg-ad-lime/60 px-2.5 py-1 text-ink">
                <span className="size-1.5 bg-ink" /> Paid
              </span>
            </div>
            <div className="px-6 pb-6">
              <p className="font-display text-[2.6rem] leading-none tracking-[-0.04em]">₦482,300</p>
              <p className="mt-2 text-[0.88rem] text-muted-foreground">Sent to your bank account ending 4821</p>
              <dl className="mt-6 divide-y text-[0.9rem]" style={{ borderColor: "var(--line)" }}>
                {[
                  ["Banner views", "614,200"],
                  ["Notification views", "62,480"],
                  ["Advertisers", "37"],
                ].map(([k, v]) => (
                  <div key={k} className="flex justify-between py-2.5" style={{ borderColor: "var(--line)" }}>
                    <dt className="text-muted-foreground">{k}</dt>
                    <dd className="font-medium">{v}</dd>
                  </div>
                ))}
              </dl>
            </div>
            <div className="bg-stone/50 px-6 py-5">
              <div className="flex justify-between text-[0.8rem]">
                <span className="text-muted-foreground">Next payout</span>
                <span className="font-medium">in 12 days</span>
              </div>
              <div className="mt-2 grid grid-cols-[repeat(30,1fr)] gap-[2px]" aria-hidden="true">
                {Array.from({ length: 30 }, (_, i) => (
                  <span key={i} className={`h-2 rounded-[1px] ${i < 18 ? "bg-ink" : "bg-stone-2"}`} />
                ))}
              </div>
              <p className="mt-3 text-[0.75rem] text-faint">Sample payout for illustration.</p>
            </div>
          </div>
        </div>
      </div>
      <ol className="mt-16 grid gap-px overflow-hidden rounded-3xl border border-[var(--line)] bg-[var(--line)] md:grid-cols-3">
            {steps.map((s) => (
              <li key={s.n} data-reveal className="bg-paper p-6">
                <span className="t-label text-faint">{s.n}</span>
                <h3 className="t-card mt-5">{s.title}</h3>
                <p className="mt-2 text-[0.92rem] leading-relaxed text-muted-foreground">{s.body}</p>
              </li>
            ))}
          </ol>
    </Section>
  );
}
