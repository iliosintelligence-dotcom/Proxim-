import { Banknote, BellOff, Eye, MapPin, ShieldCheck, Users } from "lucide-react";
import { Section } from "@/components/site/Frame";
import { AppIcon, type IconTone } from "@/components/site/AppIcon";
import type { LucideIcon } from "lucide-react";
import { HIcon } from "@/components/site/HIcon";

const items: { icon: LucideIcon; tone: IconTone; title: string; body: string }[] = [
  { icon: ShieldCheck, tone: "ink", title: "Every ad reviewed", body: "A person checks each ad before it runs, so your brand never sits next to something it shouldn’t." },
  { icon: Users, tone: "blue", title: "Real apps, real people", body: "Apps are vetted before they can show ads. Suspicious traffic is filtered out and never billed." },
  { icon: Eye, tone: "green", title: "Views that count", body: "A view is only counted when your ad is actually on screen, not loaded somewhere off-screen." },
  { icon: BellOff, tone: "pink", title: "No spam", body: "Notification ads are capped per person per week, so people keep their notifications on." },
  { icon: MapPin, tone: "orange", title: "Areas, not people", body: "Targeting works on areas and app types. Advertisers never see who a person is." },
  { icon: Banknote, tone: "yellow", title: "Pay in naira", body: "Card or bank transfer. No dollar card, no exchange-rate surprises." },
];

export function SafetySection() {
  return (
    <Section id="safety" index="04" label="Trust and safety" icon={ShieldCheck}>
      <div className="max-w-[46rem]">
        <h2 data-led className="t-section">
          Built to be <HIcon photo="1576091358783-a212ec293ff3" /> trusted on both sides.
        </h2>
        <p data-reveal className="t-lead mt-6">
          Advertisers need to know their money reaches real people. App owners need to know their users stay happy.
          These are the rules Proxim runs on.
        </p>
      </div>
      <ul className="mt-14 grid gap-px overflow-hidden rounded-3xl border border-[var(--line)] bg-[var(--line)] sm:grid-cols-2 lg:grid-cols-3">
        {items.map((it) => (
          <li key={it.title} data-reveal className="group bg-paper p-7 transition-colors duration-300 hover:bg-white">
            <AppIcon icon={it.icon} tone={it.tone} className="transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-1" />
            <h3 className="t-card mt-6 !text-[1.3rem]">{it.title}</h3>
            <p className="mt-2 text-[0.94rem] leading-relaxed text-muted-foreground">{it.body}</p>
          </li>
        ))}
      </ul>
    </Section>
  );
}
