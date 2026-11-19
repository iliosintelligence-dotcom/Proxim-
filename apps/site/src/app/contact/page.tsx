import { Code, LifeBuoy, Megaphone, MessageSquare } from "lucide-react";
import type { Metadata } from "next";
import { PageHero } from "@/components/site/PageHero";
import { Frame } from "@/components/site/Frame";
import { AppIcon, type IconTone } from "@/components/site/AppIcon";
import type { LucideIcon } from "lucide-react";
import { ContactForm } from "@/components/contact/ContactForm";
import { contact } from "@/lib/data";

export const metadata: Metadata = {
  title: "Contact: Proxim",
  description: "Talk to the Proxim team about advertising, adding Proxim to your app, or anything else.",
};

const channels: { icon: LucideIcon; tone: IconTone; title: string; body: string; value: string; href: string }[] = [
  { icon: Megaphone, tone: "blue", title: "Advertising and sales", body: "Plans, agencies and large campaigns.", value: contact.sales, href: `mailto:${contact.sales}` },
  { icon: Code, tone: "green", title: "App partnerships", body: "Adding Proxim to your app.", value: contact.partners, href: `mailto:${contact.partners}` },
  { icon: LifeBuoy, tone: "orange", title: "Help and support", body: "Questions about a live campaign.", value: contact.support, href: `mailto:${contact.support}` },
];

export default function ContactPage() {
  return (
    <>
      <PageHero
        label="Contact"
        icon={MessageSquare}
        title="Let’s talk."
        lead="Tell us what you want to promote or the app you want to earn from. A real person replies within one working day."
      />
      <section className="bg-paper">
        <Frame className="grid gap-6 pb-[var(--section-y)] lg:grid-cols-[1.35fr_1fr] lg:gap-8">
          <div data-reveal>
            <ContactForm />
          </div>
          <aside className="space-y-3">
            {channels.map((c) => (
              <a
                key={c.title}
                href={c.href}
                data-reveal
                className="group flex gap-4 rounded-3xl border border-[var(--line)] bg-stone/60 p-6 transition-colors duration-300 hover:bg-white"
              >
                <AppIcon icon={c.icon} tone={c.tone} className="transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-0.5" />
                <span className="min-w-0">
                  <span className="block font-medium">{c.title}</span>
                  <span className="block text-[0.88rem] text-muted-foreground">{c.body}</span>
                  <span className="mt-2 block truncate text-[0.84rem] font-medium text-blue">{c.value}</span>
                </span>
              </a>
            ))}
            <div data-reveal className="rounded-3xl bg-night p-6 text-white">
              <p className="t-label text-white/40">Office</p>
              <p className="mt-3 font-display text-[1.5rem] leading-tight tracking-[-0.02em]">{contact.city}</p>
              <p className="mt-2 text-[0.88rem] text-white/50">Monday to Friday, 9 AM to 6 PM (WAT)</p>
            </div>
          </aside>
        </Frame>
      </section>
    </>
  );
}
