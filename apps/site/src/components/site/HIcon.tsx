import { Photo } from "@/components/phone/Photo";

/**
 * A small photo set into a headline, pill-shaped, sitting on the text's
 * baseline. It shows the word instead of decorating it (a crowd beside
 * "audience", a street beside "walk in"), and pops in after the LED
 * text switches on.
 */
export function HIcon({ photo, alt = "" }: { photo: string; alt?: string }) {
  return (
    <span
      data-hicon
      aria-hidden={alt ? undefined : true}
      className="mx-[0.06em] inline-block h-[0.78em] w-[1.5em] translate-y-[0.06em] overflow-hidden rounded-full align-baseline shadow-[0_0.04em_0.16em_rgb(0_0_0/0.18)] ring-[0.05em] ring-white"
    >
      <Photo id={photo} w={180} h={96} alt={alt} />
    </span>
  );
}
