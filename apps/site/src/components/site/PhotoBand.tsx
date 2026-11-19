import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { Photo } from "@/components/phone/Photo";
import { Frame, SectionLabel } from "./Frame";

/**
 * A full-bleed photograph with the copy set over it. Used where the page
 * needs to stop talking about the product and show the people it reaches.
 * The photo is slightly larger than its frame and settles as you scroll
 * past (see MotionDirector [data-band-photo]), so it feels like the camera
 * pulling focus rather than a static banner.
 */
export function PhotoBand({
  photo,
  label,
  icon,
  title,
  lead,
  children,
  align = "left",
  pip,
  aside,
}: {
  pip?: string;
  /** Optional content beside the copy on wide screens (e.g. a globe). */
  aside?: ReactNode;
  photo: string;
  label: string;
  icon?: LucideIcon;
  title: ReactNode;
  lead?: string;
  children?: ReactNode;
  align?: "left" | "center";
}) {
  return (
    <section data-band data-screen data-pip={pip} className="relative overflow-clip bg-night text-white">
      <div data-band-photo className="absolute inset-0 scale-[1.18]">
        <Photo id={photo} w={1920} h={1100} />
      </div>
      <div className="absolute inset-0 bg-[linear-gradient(180deg,rgb(11_11_11/0.55)_0%,rgb(11_11_11/0.35)_40%,rgb(11_11_11/0.85)_100%)]" />
      <div className="absolute inset-0 bg-[linear-gradient(90deg,rgb(11_11_11/0.65)_0%,transparent_65%)]" />
      <Frame
        night
        className={`relative py-[clamp(5rem,12vw,10rem)] ${align === "center" ? "text-center" : ""} ${aside ? "grid items-center gap-10 lg:grid-cols-[1fr_1fr]" : ""}`}
      >
        <div>
        <SectionLabel label={label} icon={icon} night />
        <h2 data-led className={`t-display max-w-[16ch] !text-[clamp(2.4rem,6vw,5rem)] ${align === "center" ? "mx-auto" : ""}`}>
          {title}
        </h2>
        {lead && (
          <p data-reveal className={`t-lead mt-6 max-w-[34rem] !text-white/70 ${align === "center" ? "mx-auto" : ""}`}>
            {lead}
          </p>
        )}
        {children}
        </div>
        {aside}
      </Frame>
    </section>
  );
}
