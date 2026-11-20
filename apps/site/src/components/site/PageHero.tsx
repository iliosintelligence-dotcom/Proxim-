import type React from "react";
import type { ReactNode } from "react";
import { Frame } from "./Frame";
import type { LucideIcon } from "lucide-react";

/**
 * Hero for inner pages. Same parts as the home heroes (label, headline,
 * lead) so every page opens the same way. It can sit centred or left,
 * and can carry a ticker that runs between the page's two border lines.
 */
export function PageHero({
  label,
  icon: Icon,
  title,
  lead,
  children,
  align = "center",
  ticker,
}: {
  label: string;
  icon: LucideIcon;
  title: React.ReactNode;
  lead: string;
  children?: ReactNode;
  align?: "center" | "left";
  /** Rendered under the copy, from one border line to the other. */
  ticker?: ReactNode;
}) {
  const left = align === "left";
  return (
    <section className="relative overflow-clip bg-paper">
      <Frame
        className={`pt-[clamp(2.75rem,6.5vw,5rem)] ${ticker ? "pb-[clamp(2rem,4vw,3rem)]" : "pb-[clamp(3.5rem,8vw,6rem)]"} ${left ? "text-left" : "text-center"}`}
      >
        <p data-reveal className="t-label inline-flex items-center gap-2 text-ink">
          <Icon className="size-3.5 text-blue" strokeWidth={2.2} />/ {label}
        </p>
        <h1 data-led className={`t-display mt-6 max-w-[19ch] ${left ? "" : "mx-auto"}`}>
          {title}
        </h1>
        <p data-reveal className={`t-lead mt-5 max-w-[40rem] ${left ? "" : "mx-auto"}`}>
          {lead}
        </p>
        {children && (
          <div data-reveal className="mt-9">
            {children}
          </div>
        )}
      </Frame>
      {ticker && (
        // Same column as the copy, minus its side padding, so the ticker
        // starts and stops exactly on the border lines.
        <Frame className="overflow-x-clip !px-0 pb-[clamp(3rem,7vw,5rem)]">
          <div data-reveal>{ticker}</div>
        </Frame>
      )}
    </section>
  );
}
