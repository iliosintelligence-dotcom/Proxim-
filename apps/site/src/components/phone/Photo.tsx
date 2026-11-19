/* eslint-disable @next/next/no-img-element */
import { photo } from "@/lib/data";

/**
 * A self-hosted Unsplash photo, cropped to its box with object-fit.
 * Served from the site itself, so no request leaves the domain.
 */
export function Photo({
  id,
  w,
  h,
  alt = "",
  className = "",
}: {
  id: string;
  w: number;
  h: number;
  alt?: string;
  className?: string;
}) {
  return (
    <img
      src={photo(id, w, h)}
      alt={alt}
      loading="lazy"
      decoding="async"
      draggable={false}
      className={`block h-full w-full object-cover ${className}`}
    />
  );
}
