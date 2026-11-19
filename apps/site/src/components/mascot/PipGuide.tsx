"use client";

import { useEffect, useRef, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { X } from "lucide-react";
import { gsap, ScrollTrigger, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { Pip } from "./Pip";

/**
 * Pip as a guide. Once you're past the hero, Pip waits in the corner and
 * says one useful thing about the section you're reading (each section
 * sets its line with data-pip). The bubble shows for a few seconds when
 * the line changes, then tucks away so it never covers content for long.
 * Clicking Pip takes you to the page's main action. It can be dismissed.
 */
export function PipGuide() {
  const pathname = usePathname();
  const router = useRouter();
  const root = useRef<HTMLDivElement>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [bubble, setBubble] = useState(false);
  const [visible, setVisible] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  // The footer has its own Pip, so the guide steps aside there.
  const [atFooter, setAtFooter] = useState(false);
  // ...and while the cookie notice is open, so the two never overlap.
  const [cookieOpen, setCookieOpen] = useState(false);
  useEffect(() => {
    const on = () => setCookieOpen(true);
    const off = () => setCookieOpen(false);
    window.addEventListener("cookies:open", on);
    window.addEventListener("cookies:close", off);
    return () => {
      window.removeEventListener("cookies:open", on);
      window.removeEventListener("cookies:close", off);
    };
  }, []);
  const hideTimer = useRef<number | null>(null);

  useEffect(() => {
    try {
      setDismissed(sessionStorage.getItem("pip-dismissed") === "1");
    } catch {
      /* storage unavailable: just show Pip */
    }
  }, []);

  // Watch the sections on this page for Pip's lines.
  useEffect(() => {
    setMessage(null);
    const triggers: ScrollTrigger[] = [];
    const id = window.setTimeout(() => {
      triggers.push(
        ScrollTrigger.create({
          start: () => window.innerHeight * 0.75,
          onToggle: (s) => setVisible(s.isActive),
        }),
      );
      const footer = document.querySelector("footer");
      if (footer)
        triggers.push(
          ScrollTrigger.create({ trigger: footer, start: "top 85%", onToggle: (s) => setAtFooter(s.isActive) }),
        );
      document.querySelectorAll<HTMLElement>("[data-pip]").forEach((el) => {
        triggers.push(
          ScrollTrigger.create({
            trigger: el,
            start: "top 55%",
            end: "bottom 55%",
            onToggle: (s) => s.isActive && setMessage(el.dataset.pip ?? null),
          }),
        );
      });
    }, 400);
    return () => {
      window.clearTimeout(id);
      triggers.forEach((t) => t.kill());
    };
  }, [pathname]);

  // Show the bubble for a moment whenever the line changes.
  useEffect(() => {
    if (!message) return;
    setBubble(true);
    if (hideTimer.current) window.clearTimeout(hideTimer.current);
    hideTimer.current = window.setTimeout(() => setBubble(false), 5200);
    return () => {
      if (hideTimer.current) window.clearTimeout(hideTimer.current);
    };
  }, [message]);

  useGSAP(
    () => {
      const el = root.current!;
      const show = visible && !dismissed && !atFooter && !cookieOpen;
      gsap.to(el, {
        y: show ? 0 : 40,
        autoAlpha: show ? 1 : 0,
        duration: prefersReducedMotion() ? 0 : 0.6,
        ease: show ? "back.out(2)" : "power2.in",
      });
    },
    { dependencies: [visible, dismissed, atFooter, cookieOpen], scope: root },
  );

  useGSAP(
    () => {
      const b = root.current!.querySelector("[data-bubble]");
      if (!b) return;
      if (bubble && !prefersReducedMotion()) {
        gsap.fromTo(b, { autoAlpha: 0, y: 6, scale: 0.96 }, { autoAlpha: 1, y: 0, scale: 1, duration: 0.45, ease: "back.out(2)" });
      } else {
        gsap.to(b, { autoAlpha: bubble ? 1 : 0, duration: 0.25 });
      }
    },
    { dependencies: [bubble, message], scope: root },
  );

  const cta = pathname.startsWith("/publishers") ? "/publishers#start" : pathname === "/" ? "#launch" : "/#launch";

  return (
    <div
      ref={root}
      className="invisible fixed bottom-4 right-4 z-40 flex items-end gap-2 opacity-0 sm:bottom-6 sm:right-6"
      aria-live="polite"
    >
      <div
        data-bubble
        className="invisible relative mb-6 max-w-[14rem] rounded-2xl rounded-br-md border border-[var(--line)] bg-white px-3.5 py-2.5 text-[0.85rem] leading-snug text-ink opacity-0 shadow-[0_14px_36px_-14px_rgb(28_25_23/0.4)] sm:max-w-[16rem]"
      >
        {message}
        <button
          type="button"
          aria-label="Hide Pip"
          onClick={() => {
            setDismissed(true);
            try {
              sessionStorage.setItem("pip-dismissed", "1");
            } catch {
              /* ignore */
            }
          }}
          className="absolute -right-2 -top-2 grid size-5 place-items-center rounded-full border border-[var(--line)] bg-white text-muted-foreground hover:text-ink"
        >
          <X className="size-3" />
        </button>
      </div>
      <button
        type="button"
        onClick={() => (cta.startsWith("#") ? document.querySelector(cta)?.scrollIntoView({ behavior: "smooth" }) : router.push(cta))}
        onPointerEnter={() => setBubble(true)}
        aria-label={message ? `Pip says: ${message}` : "Pip, take me to get started"}
        className="rounded-full focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-blue"
      >
        <Pip size={52} />
      </button>
    </div>
  );
}
