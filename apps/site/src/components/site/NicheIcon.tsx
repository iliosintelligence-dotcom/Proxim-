import type { Niche } from "@/lib/data";
import { Photo } from "@/components/phone/Photo";

const sizes = {
  xs: "size-5",
  sm: "size-8",
  md: "size-11",
};

/**
 * A niche, shown as a round photo of the kind of business that advertises
 * in it (worship for Faith, a gym session for Fitness, jollof for Food).
 * Used everywhere a niche is listed so each one is recognisable at a glance.
 */
export function NicheIcon({
  niche,
  size = "md",
  muted = false,
  className = "",
}: {
  niche: Niche;
  size?: keyof typeof sizes;
  muted?: boolean;
  className?: string;
}) {
  return (
    <span
      data-niche-icon
      aria-hidden="true"
      className={`block shrink-0 overflow-hidden rounded-full shadow-[0_1px_3px_rgb(0_0_0/0.15)] ring-2 ring-white transition-[filter,opacity] duration-300 ${sizes[size]} ${
        muted ? "opacity-50 grayscale" : ""
      } ${className}`}
    >
      <Photo id={niche.ad.photo} w={96} h={96} />
    </span>
  );
}
