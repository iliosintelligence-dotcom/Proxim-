"use client";

import { Bell, CalendarDays, ChartColumn, Crosshair, MapPin, ShieldCheck } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { lockScroll } from "@/lib/lenis";
import { nicheById } from "@/lib/data";
import { Logo } from "./Logo";
import { Button, PixelArrow } from "./Button";
import { AppIcon, type IconTone } from "./AppIcon";
import type { LucideIcon } from "lucide-react";
import { NotificationAd } from "@/components/phone/Ads";

const links = [
  { label: "Features", href: "/features", menu: true },
  { label: "Pricing", href: "/pricing" },
  { label: "App owners", href: "/publishers" },
  { label: "About", href: "/about" },
  { label: "Contact", href: "/contact" },
];

const featureItems: { icon: LucideIcon; tone: IconTone; title: string; body: string; href: string }[] = [
  { icon: Crosshair, tone: "blue", title: "Niche matching", body: "Only in apps that fit what you sell", href: "/features#targeting" },
  { icon: MapPin, tone: "orange", title: "Proximity", body: "From 2 km to your whole state", href: "/features#targeting" },
  { icon: Bell, tone: "pink", title: "Banners and notifications", body: "In the app or on the lock screen", href: "/features#formats" },
  { icon: CalendarDays, tone: "purple", title: "Scheduling", body: "Run it for a weekend or a season", href: "/features#scheduling" },
  { icon: ChartColumn, tone: "green", title: "Live reporting", body: "Every view and tap as it happens", href: "/features#reporting" },
  { icon: ShieldCheck, tone: "ink", title: "Brand safety", body: "Real apps, reviewed ads", href: "/features#safety" },
];

/**
 * The navigation behaves like the product. At the top of a page it's a
 * wide, quiet bar. Once you scroll it gathers into a floating capsule,
 * the way Proxim narrows a broad audience down to the people who matter.
 * A small blue pixel marks where you are and follows the pointer, like
 * the pin on the map.
 */
export function Nav() {
  const pathname = usePathname();
  const [compact, setCompact] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [sheetOpen, setSheetOpen] = useState(false);
  const root = useRef<HTMLElement>(null);
  const linkRefs = useRef<(HTMLAnchorElement | null)[]>([]);
  const closeTimer = useRef<number | null>(null);

  const activeIndex = links.findIndex((l) => pathname.startsWith(l.href));
  const cta = pathname.startsWith("/publishers")
    ? { href: "/publishers#start", label: "Start earning" }
    : { href: "/#launch", label: "Launch a campaign" };

  useEffect(() => {
    const onScroll = () => setCompact(window.scrollY > 24);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  // Close everything when the route changes.
  useEffect(() => {
    setMenuOpen(false);
    setSheetOpen(false);
  }, [pathname]);

  useEffect(() => {
    lockScroll(sheetOpen);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setMenuOpen(false);
        setSheetOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [sheetOpen]);

  const { contextSafe } = useGSAP({ scope: root });

  // The "you are here" pixel.
  const moveMarker = contextSafe((index: number) => {
    const marker = root.current?.querySelector<HTMLElement>("[data-marker]");
    const el = linkRefs.current[index];
    if (!marker) return;
    if (!el) {
      gsap.to(marker, { autoAlpha: 0, scale: 0.4, duration: 0.25 });
      return;
    }
    const x = el.offsetLeft + el.offsetWidth / 2 - 3;
    gsap.to(marker, {
      x,
      autoAlpha: 1,
      scale: 1,
      duration: prefersReducedMotion() ? 0 : 0.55,
      ease: "expo.out",
    });
  });

  // While the Features panel is open the pixel stays on Features.
  useEffect(() => {
    const id = requestAnimationFrame(() => moveMarker(menuOpen ? 0 : activeIndex));
    return () => cancelAnimationFrame(id);
  }, [activeIndex, compact, menuOpen, moveMarker]);

  // Features panel.
  useGSAP(
    () => {
      const panel = root.current!.querySelector<HTMLElement>("[data-panel]");
      if (!panel) return;
      gsap.set(panel, { xPercent: -50 });
      if (menuOpen) {
        gsap.fromTo(
          panel,
          { autoAlpha: 0, y: -8, scale: 0.985 },
          { autoAlpha: 1, y: 0, scale: 1, duration: 0.45, ease: "expo.out" },
        );
        gsap.fromTo(
          panel.querySelectorAll("[data-panel-item]"),
          { autoAlpha: 0, y: 6 },
          { autoAlpha: 1, y: 0, duration: 0.5, stagger: 0.03, ease: "expo.out", delay: 0.05 },
        );
      } else {
        gsap.to(panel, { autoAlpha: 0, y: -4, duration: 0.18, ease: "power2.in" });
      }
    },
    { scope: root, dependencies: [menuOpen] },
  );

  // Phone menu: wipes down, then each link switches on like an LED sign.
  useGSAP(
    () => {
      const sheet = root.current!.querySelector<HTMLElement>("[data-sheet]");
      if (!sheet) return;
      const reduce = prefersReducedMotion();
      if (sheetOpen) {
        gsap.fromTo(
          sheet,
          { autoAlpha: 1, clipPath: "inset(0% 0% 100% 0%)" },
          { clipPath: "inset(0% 0% 0% 0%)", duration: reduce ? 0 : 0.6, ease: "expo.out" },
        );
        if (!reduce)
          gsap.fromTo(
            sheet.querySelectorAll("[data-sheet-item]"),
            { opacity: 0 },
            {
              keyframes: { opacity: [0, 1, 0.25, 1], easeEach: "steps(1)" },
              duration: 0.35,
              ease: "none",
              stagger: 0.06,
              delay: 0.15,
            },
          );
      } else {
        gsap.to(sheet, {
          clipPath: "inset(0% 0% 100% 0%)",
          duration: reduce ? 0 : 0.45,
          ease: "expo.inOut",
          onComplete: () => gsap.set(sheet, { autoAlpha: 0 }),
        });
      }
    },
    { scope: root, dependencies: [sheetOpen] },
  );

  const openMenu = useCallback(() => {
    if (closeTimer.current) window.clearTimeout(closeTimer.current);
    setMenuOpen(true);
  }, []);
  const closeMenuSoon = useCallback(() => {
    if (closeTimer.current) window.clearTimeout(closeTimer.current);
    closeTimer.current = window.setTimeout(() => setMenuOpen(false), 140);
  }, []);

  const floating = compact || sheetOpen;

  return (
    <header ref={root} className="sticky top-0 z-50 px-2 pt-2 sm:px-3 sm:pt-3">
      <div
        className={`relative mx-auto flex items-center gap-2 transition-[max-width,height,border-radius,background-color,box-shadow,padding] duration-700 ease-[var(--ease-out-quint)] ${
          floating
            ? "h-14 max-w-[1040px] rounded-2xl bg-white pl-4 pr-2 shadow-[0_0_0_1px_var(--line),0_12px_32px_-18px_rgb(13_13_13/0.25)] sm:pl-5"
            : "h-16 max-w-[1240px] rounded-2xl bg-transparent px-[calc(var(--gutter)-0.5rem)] sm:px-[calc(var(--gutter)-0.75rem)]"
        }`}
      >
        <Link href="/" aria-label="Proxim home" className="relative z-10 shrink-0 rounded-md focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-blue">
          <Logo />
        </Link>

        {/* Desktop links */}
        <nav aria-label="Main" className="relative mx-auto hidden lg:block" onPointerLeave={() => !menuOpen && moveMarker(activeIndex)}>
          <ul className="flex items-center">
            {links.map((l, i) => {
              const active = i === activeIndex;
              return (
                <li
                  key={l.href}
                  onPointerEnter={l.menu ? openMenu : undefined}
                  onPointerLeave={l.menu ? closeMenuSoon : undefined}
                >
                  <Link
                    ref={(el) => {
                      linkRefs.current[i] = el;
                    }}
                    href={l.href}
                    aria-current={active ? "page" : undefined}
                    aria-expanded={l.menu ? menuOpen : undefined}
                    aria-haspopup={l.menu ? "true" : undefined}
                    onPointerEnter={() => moveMarker(i)}
                    onFocus={() => moveMarker(i)}
                    onClick={(e) => {
                      if (l.menu && !menuOpen && e.detail === 0) {
                        e.preventDefault();
                        setMenuOpen(true);
                      }
                    }}
                    className={`group inline-flex items-center gap-1.5 rounded-full px-3.5 py-2 text-[0.9rem] font-medium transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-blue ${
                      active ? "text-ink" : "text-ink-2/75 hover:text-ink"
                    }`}
                  >
                    {l.label}
                    {l.menu && (
                      <svg
                        viewBox="0 0 7 4"
                        className={`h-1 w-auto transition-transform duration-300 ${menuOpen ? "rotate-180" : ""}`}
                        shapeRendering="crispEdges"
                        aria-hidden="true"
                      >
                        <path d="M0 0h1v1H0zM1 1h1v1H1zM2 2h1v1H2zM3 3h1v1H3zM4 2h1v1H4zM5 1h1v1H5zM6 0h1v1H6z" fill="currentColor" />
                      </svg>
                    )}
                  </Link>
                </li>
              );
            })}
          </ul>
          <span
            data-marker
            aria-hidden="true"
            className="invisible absolute -bottom-0.5 left-0 size-[6px] bg-blue"
          />
        </nav>

        <div className="relative z-10 ml-auto flex shrink-0 items-center gap-1.5 lg:ml-0">
          <Link
            href="#"
            className="hidden rounded-full px-3.5 py-2 text-[0.9rem] font-medium text-ink-2/75 transition-colors hover:text-ink md:inline-block"
          >
            Log in
          </Link>
          <span className="hidden sm:block">
            <Button href={cta.href} size="sm" arrow>
              {cta.label}
            </Button>
          </span>
          <button
            type="button"
            aria-label={sheetOpen ? "Close menu" : "Open menu"}
            aria-expanded={sheetOpen}
            onClick={() => setSheetOpen((v) => !v)}
            className="grid size-10 place-items-center rounded-full text-ink transition-colors hover:bg-stone focus-visible:outline-2 focus-visible:outline-blue lg:hidden"
          >
            <svg viewBox="0 0 9 9" className="size-[15px]" shapeRendering="crispEdges" aria-hidden="true">
              {sheetOpen
                ? [0, 1, 2, 3, 4, 5, 6, 7, 8].flatMap((i) => [
                    <rect key={`a${i}`} x={i} y={i} width="1" height="1" fill="currentColor" />,
                    <rect key={`b${i}`} x={8 - i} y={i} width="1" height="1" fill="currentColor" />,
                  ])
                : [1, 4, 7].map((y) => <rect key={y} x="0" y={y} width={y === 4 ? 6 : 9} height="1.2" fill="currentColor" />)}
            </svg>
          </button>
        </div>

        {/* Features panel */}
        <div
          data-panel
          data-ad-manual
          onPointerEnter={openMenu}
          onPointerLeave={closeMenuSoon}
          className="invisible absolute left-1/2 top-[calc(100%+0.5rem)] hidden w-[min(54rem,calc(100vw-2rem))] grid-cols-[1fr_17rem] gap-2 rounded-3xl border border-[var(--line)] bg-paper/95 p-2 shadow-[0_24px_60px_-20px_rgb(28_25_23/0.35)] backdrop-blur-xl lg:grid"
        >
          <ul className="grid grid-cols-2 gap-1 p-2">
            {featureItems.map((f) => (
              <li key={f.title} data-panel-item>
                <Link
                  href={f.href}
                  onClick={() => setMenuOpen(false)}
                  className="group flex gap-3.5 rounded-2xl p-3.5 transition-colors duration-200 hover:bg-stone focus-visible:outline-2 focus-visible:outline-blue"
                >
                  <AppIcon icon={f.icon} tone={f.tone} className="!size-10 transition-transform duration-300 group-hover:-translate-y-0.5" />
                  <span>
                    <span className="block text-[0.92rem] font-medium">{f.title}</span>
                    <span className="block text-[0.82rem] leading-snug text-muted-foreground">{f.body}</span>
                  </span>
                </Link>
              </li>
            ))}
          </ul>
          <Link
            href="/#formats"
            data-panel-item
            onClick={() => setMenuOpen(false)}
            className="group relative flex flex-col overflow-hidden rounded-[1.1rem] bg-night p-5 text-white"
          >
            <span className="t-label text-white/40">See it working</span>
            <span className="mt-2 font-display text-[1.25rem] leading-tight tracking-[-0.02em]">Watch an ad land</span>
            <span className="ios-scale mt-auto block pt-6 transition-transform duration-500 ease-[var(--ease-out-quint)] group-hover:-translate-y-1">
              <NotificationAd niche={nicheById("fitness")} className="!visible" />
            </span>
            <span className="mt-4 inline-flex items-center gap-2 text-[0.84rem] text-white/70 transition-colors group-hover:text-white">
              Try the live demo <PixelArrow />
            </span>
          </Link>
        </div>
      </div>

      {/* Phone and tablet menu */}
      <div
        data-sheet
        className="invisible fixed inset-0 -z-10 flex flex-col bg-paper px-[var(--gutter)] pb-8 pt-24 lg:hidden"
        aria-hidden={!sheetOpen}
      >
        <nav aria-label="Mobile">
          <ul>
            {[{ label: "Home", href: "/" }, ...links].map((l) => (
              <li key={l.href} data-sheet-item className="border-b" style={{ borderColor: "var(--line)" }}>
                <Link
                  href={l.href}
                  tabIndex={sheetOpen ? 0 : -1}
                  onClick={() => setSheetOpen(false)}
                  className="flex items-center justify-between py-4 font-display text-[clamp(1.8rem,8vw,2.6rem)] leading-none tracking-[-0.03em]"
                >
                  {l.label}
                  <PixelArrow className="!size-5 text-faint" />
                </Link>
              </li>
            ))}
          </ul>
        </nav>
        <div data-sheet-item className="mt-auto grid gap-3 sm:grid-cols-2">
          <Button href={cta.href} arrow className="w-full">
            {cta.label}
          </Button>
          <Button href="#" variant="secondary" className="w-full">
            Log in
          </Button>
        </div>
      </div>
    </header>
  );
}
