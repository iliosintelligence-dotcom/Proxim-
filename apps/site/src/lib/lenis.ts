import type Lenis from "lenis";

/** Shared handle on the smooth-scroll instance, so menus can pause it. */
let instance: Lenis | null = null;

export const setLenis = (l: Lenis | null) => {
  instance = l;
};

export const lockScroll = (locked: boolean) => {
  if (instance) {
    if (locked) instance.stop();
    else instance.start();
  }
  document.documentElement.style.overflow = locked ? "hidden" : "";
};

export const scrollToTop = () => instance?.scrollTo(0, { immediate: true });

/** Glide back to the top (falls back to native smooth scroll). */
export const glideToTop = () => {
  if (instance) instance.scrollTo(0, { duration: 1.6 });
  else window.scrollTo({ top: 0, behavior: "smooth" });
};
