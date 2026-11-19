import { Smartphone } from "lucide-react";
import { advertiserCpm, nicheById } from "@/lib/data";
import { naira } from "@/lib/format";
import { Section } from "@/components/site/Frame";
import { LivePhone } from "@/components/phone/LivePhone";
import { HIcon } from "@/components/site/HIcon";

const rows: { label: string; banner: string; notification: string }[] = [
  { label: "Where it appears", banner: "Inside the app, above the home bar", notification: "On the lock screen and in the notification list" },
  { label: "When it’s seen", banner: "While someone uses the app", notification: "When they’re inside your radius, even with the app closed" },
  { label: "What you include", banner: "Logo, headline, one line, button", notification: "Logo, headline, one line" },
  { label: "Where taps go", banner: "Your website, map pin, WhatsApp or call", notification: "Your website, map pin, WhatsApp or call" },
  { label: "Best for", banner: "Steady awareness", notification: "Today-only offers and events" },
  { label: "From", banner: `${naira(advertiserCpm.banner)} per 1,000 views`, notification: `${naira(advertiserCpm.notification)} per 1,000 views` },
];

export function FormatSpecs() {
  const niche = nicheById("food");
  return (
    <Section id="formats" index="02" label="Formats" tone="stone" icon={Smartphone}>
      <div className="max-w-[46rem]">
        <h2 data-led className="t-section">
          Two <HIcon photo="1709837167686-a2e33aad1bf0" /> formats, side by side.
        </h2>
        <p data-reveal className="t-lead mt-6">
          Both formats carry the same ad. What changes is where it shows up and how loud it is.
        </p>
      </div>

      <div data-reveal className="mt-14 overflow-hidden rounded-3xl border border-[var(--line)] bg-white">
        <div className="grid grid-cols-2 border-b md:grid-cols-[14rem_1fr_1fr]" style={{ borderColor: "var(--line)" }}>
          <div className="hidden md:block" />
          {(["banner", "notification"] as const).map((f) => (
            <div key={f} className="border-l p-5 first:border-l-0 md:first:border-l sm:p-6" style={{ borderColor: "var(--line)" }}>
              <p className="t-card">{f === "banner" ? "Banner" : "Notification"}</p>
              <div className="relative mt-5 h-[13rem] overflow-hidden rounded-2xl bg-stone sm:h-[16rem]">
                <LivePhone
                  niche={niche}
                  format={f}
                  screen={f === "notification" ? "lock" : "app"}
                  className={`absolute left-1/2 w-[9.5rem] -translate-x-1/2 sm:w-[11.5rem] ${f === "banner" ? "-bottom-3" : "top-4"}`}
                />
              </div>
            </div>
          ))}
        </div>
        <dl>
          {rows.map((r) => (
            <div
              key={r.label}
              className="grid grid-cols-2 border-b last:border-b-0 md:grid-cols-[14rem_1fr_1fr]"
              style={{ borderColor: "var(--line)" }}
            >
              <dt className="col-span-2 px-5 pt-4 text-[0.8rem] font-medium text-muted-foreground md:col-span-1 md:p-6 md:text-[0.9rem] md:text-ink">
                {r.label}
              </dt>
              <dd className="px-5 pb-4 pt-1.5 text-[0.9rem] md:border-l md:p-6" style={{ borderColor: "var(--line)" }}>
                {r.banner}
              </dd>
              <dd className="border-l px-5 pb-4 pt-1.5 text-[0.9rem] md:p-6" style={{ borderColor: "var(--line)" }}>
                {r.notification}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </Section>
  );
}
