"use client";

import { useEffect, useRef } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { PIP_BLUE, PIP_BODY } from "./PipShape";

/*
  Pip: Proxim's mascot and logo. A smooth blue map pin with a face, built
  in the same heavy, rounded geometry as the display type. Pip's job is
  the product: carrying ads to people nearby. Each time an ad goes out,
  Pip pings (rings ripple from its point, like a location ping on a map).

  Behaviour, all optional:
  - blinks every few seconds
  - eyes follow the pointer
  - bobs gently
  - `pingKey` changes   -> ping and a little hop
  - `look` (-1, 0, 1)   -> eyes turn toward something
  - window "pip:happy"  -> happy eyes and a hop (e.g. when a price updates)
*/

export function Pip({
  size = 64,
  pingKey = 0,
  look = 0,
  className = "",
  title = "Pip, the Proxim mascot",
}: {
  size?: number;
  pingKey?: number;
  look?: -1 | 0 | 1;
  className?: string;
  title?: string;
}) {
  const root = useRef<HTMLDivElement>(null);
  const lookRef = useRef(look);
  lookRef.current = look;

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      const q = gsap.utils.selector(root);
      gsap.to(q("[data-pip-figure]"), { y: -2.2, duration: 1.2, ease: "sine.inOut", yoyo: true, repeat: -1 });

      let timer = gsap.delayedCall(1.5 + Math.random() * 2, blink);
      function blink() {
        gsap
          .timeline()
          .to(q("[data-eyes-open]"), { scaleY: 0.08, transformOrigin: "50% 60%", duration: 0.07, ease: "power2.in" })
          .set(q("[data-eyes-closed]"), { opacity: 1 })
          .set(q("[data-eyes-open]"), { opacity: 0 })
          .set(q("[data-eyes-closed]"), { opacity: 0 }, "+=0.08")
          .set(q("[data-eyes-open]"), { opacity: 1 })
          .to(q("[data-eyes-open]"), { scaleY: 1, duration: 0.12, ease: "power2.out" });
        timer = gsap.delayedCall(2.5 + Math.random() * 3, blink);
      }

      const xTo = gsap.quickTo(q("[data-pupils]"), "x", { duration: 0.3, ease: "power2.out" });
      const yTo = gsap.quickTo(q("[data-pupils]"), "y", { duration: 0.3, ease: "power2.out" });
      const onMove = (e: PointerEvent) => {
        if (lookRef.current !== 0) return;
        const r = root.current!.getBoundingClientRect();
        const dx = e.clientX - (r.left + r.width / 2);
        const dy = e.clientY - (r.top + r.height / 2);
        const d = Math.hypot(dx, dy) || 1;
        xTo((dx / d) * 1.8);
        yTo((dy / d) * 1.4);
      };
      window.addEventListener("pointermove", onMove);

      const onHappy = () => {
        gsap
          .timeline()
          .set(q("[data-eyes-open]"), { opacity: 0 })
          .set(q("[data-eyes-happy]"), { opacity: 1 })
          .to(q("[data-pip-figure]"), { y: -7, duration: 0.18, ease: "power2.out" }, 0)
          .to(q("[data-pip-figure]"), { y: 0, duration: 0.55, ease: "bounce.out" })
          .set(q("[data-eyes-open]"), { opacity: 1 }, 1.1)
          .set(q("[data-eyes-happy]"), { opacity: 0 }, 1.1);
      };
      window.addEventListener("pip:happy", onHappy);
      return () => {
        timer.kill();
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pip:happy", onHappy);
      };
    },
    { scope: root },
  );

  useEffect(() => {
    if (look === 0 || !root.current) return;
    gsap.to(root.current.querySelector("[data-pupils]"), { x: look * 1.8, y: 0, duration: 0.3, ease: "power2.out" });
  }, [look]);

  useEffect(() => {
    if (!pingKey || !root.current || prefersReducedMotion()) return;
    const q = gsap.utils.selector(root);
    gsap.fromTo(
      q("[data-ring]"),
      { attr: { rx: 2, ry: 0.8 }, opacity: 0.9 },
      { attr: { rx: 26, ry: 8 }, opacity: 0, duration: 1.1, ease: "power2.out", stagger: 0.18 },
    );
    gsap.fromTo(q("[data-pip-figure]"), { y: 0 }, { y: -5, duration: 0.16, yoyo: true, repeat: 1, ease: "power2.out" });
  }, [pingKey]);

  return (
    <div ref={root} className={`relative inline-block ${className}`} style={{ width: size, height: size * 1.32 }}>
      <svg viewBox="-6 -2 76 92" className="size-full overflow-visible" role="img" aria-label={title}>
        <ellipse cx="32" cy="80" rx="10" ry="2.6" fill="rgb(13 13 13 / 0.14)" />
        {[0, 1, 2].map((i) => (
          <ellipse key={i} data-ring cx="32" cy="80" rx="2" ry="0.8" fill="none" stroke={PIP_BLUE} strokeWidth="1.1" opacity="0" />
        ))}
        <g data-pip-figure>
          <path d={PIP_BODY} fill={PIP_BLUE} />
          <ellipse cx="20" cy="17" rx="7" ry="4.5" fill="#fff" opacity=".22" transform="rotate(-30 20 17)" />
          <g data-eyes-open>
            <ellipse cx="23" cy="30" rx="6.6" ry="8.2" fill="#fff" />
            <ellipse cx="41" cy="30" rx="6.6" ry="8.2" fill="#fff" />
            <g data-pupils>
              <circle cx="24.2" cy="31.4" r="3.6" fill="#0d0d0d" />
              <circle cx="42.2" cy="31.4" r="3.6" fill="#0d0d0d" />
              <circle cx="25.4" cy="30" r="1.1" fill="#fff" />
              <circle cx="43.4" cy="30" r="1.1" fill="#fff" />
            </g>
          </g>
          <g data-eyes-closed opacity="0" fill="none" stroke="#0d0d0d" strokeWidth="2.6" strokeLinecap="round">
            <path d="M17.5 31q5.5 3 11 0" />
            <path d="M35.5 31q5.5 3 11 0" />
          </g>
          <g data-eyes-happy opacity="0" fill="none" stroke="#0d0d0d" strokeWidth="2.8" strokeLinecap="round">
            <path d="M17.5 32q5.5-6.5 11 0" />
            <path d="M35.5 32q5.5-6.5 11 0" />
          </g>
          <ellipse cx="15.5" cy="41" rx="3.6" ry="2.3" fill="#ff9ec0" />
          <ellipse cx="48.5" cy="41" rx="3.6" ry="2.3" fill="#ff9ec0" />
          <path d="M26.5 43.5q5.5 5 11 0" fill="none" stroke="#0b1b4a" strokeWidth="3" strokeLinecap="round" />
        </g>
      </svg>
    </div>
  );
}
