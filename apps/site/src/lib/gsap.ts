"use client";

import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { SplitText } from "gsap/SplitText";
import { MotionPathPlugin } from "gsap/MotionPathPlugin";
import { useEffect, useLayoutEffect, useRef } from "react";
import type React from "react";

if (typeof window !== "undefined") {
  gsap.registerPlugin(ScrollTrigger, SplitText, MotionPathPlugin);
  gsap.defaults({ ease: "expo.out", duration: 0.9 });
}

export const prefersReducedMotion = () =>
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/*
  Leaving a page fast.

  Normally every animation is reverted when its component goes away: GSAP
  puts each element's original styles back, one by one, forcing the
  browser to recalculate the page's styles each time. When the whole page
  is about to be thrown away that work is wasted, and on a phone it added
  seconds to every tap in the menu. So while a page change is underway,
  animations are stopped where they stand instead, and only their cleanup
  functions (event listeners and the like) still run.
*/
let leavingPage = false;

/*
  While the next page loads, the current one keeps animating underneath the
  phone menu: tickers, loops, Pip. On a phone each of those frames cost
  ~60ms of style work, starving the new page of the main thread, so a tap
  took seconds. Freezing GSAP's clock hands the whole thread to the new
  page; it wakes by itself the moment the new page creates its first
  animation (and after a few seconds as a safety net).
*/
function freezeForNavigation() {
  leavingPage = true;
  gsap.ticker.sleep();
  const from = window.location.pathname;
  window.setTimeout(() => {
    if (window.location.pathname === from) {
      leavingPage = false;
      gsap.ticker.wake();
    }
  }, 8000);
}

if (typeof window !== "undefined") {
  document.addEventListener(
    "click",
    (e) => {
      if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const a = (e.target as Element | null)?.closest?.("a[href]") as HTMLAnchorElement | null;
      if (!a || (a.target && a.target !== "_self")) return;
      const url = new URL(a.href, window.location.href);
      if (url.origin === window.location.origin && url.pathname !== window.location.pathname) freezeForNavigation();
    },
    true,
  );
  window.addEventListener("popstate", () => {
    leavingPage = true;
  });
}

type Ctx = gsap.Context & { _r?: ((revert: boolean, ctx: gsap.Context) => void)[] };

/** Stop a context's animations without restoring any styles, then run its cleanups. */
export function dropContext(ctx: gsap.Context) {
  const cleanups = [...((ctx as Ctx)._r ?? [])];
  ctx.kill();
  cleanups.forEach((f) => f(false, ctx));
}

const useIso = typeof document !== "undefined" ? useLayoutEffect : useEffect;
const none: unknown[] = [];
type Callback = (context: gsap.Context, contextSafe: gsap.ContextSafeFunc) => void | (() => void);
type Config = {
  scope?: React.RefObject<Element | null>;
  dependencies?: unknown[];
  revertOnUpdate?: boolean;
  /** Set up only once the scope is within a screen of the viewport. */
  lazy?: boolean;
};

/**
 * Same API as @gsap/react's useGSAP (scoped context, contextSafe, optional
 * dependencies), plus two things for phones:
 *  - when the page is being left, animations are dropped instead of reverted;
 *  - with `lazy`, a section's animations are only set up once it comes
 *    within a screen of the viewport, so a page's first paint isn't held
 *    up by sections far below it.
 */
export function useGSAP(callback?: Callback | Config, dependencies: unknown[] | Config = none) {
  let config: Config = {};
  let fn: Callback | undefined;
  let deps = dependencies as unknown[];
  if (callback && typeof callback === "object") {
    config = callback;
    deps = config.dependencies ?? none;
  } else {
    fn = callback;
    if (dependencies && !Array.isArray(dependencies)) {
      config = dependencies;
      deps = config.dependencies ?? none;
    }
  }
  const { scope, revertOnUpdate, lazy = false } = config;
  const mounted = useRef(false);
  const context = useRef<gsap.Context>(null as unknown as gsap.Context);
  if (!context.current) context.current = gsap.context(() => {}, scope as unknown as Element);
  const contextSafe = useRef<gsap.ContextSafeFunc>(((f: (...args: unknown[]) => unknown) =>
    context.current.add(null as unknown as string, f)) as gsap.ContextSafeFunc);
  const deferCleanup = deps.length > 0 && !revertOnUpdate;

  const teardown = () => {
    if (leavingPage) dropContext(context.current);
    else context.current.revert();
  };

  // Lazy sections wait for the scope to come near; the latest callback runs.
  const active = useRef(!lazy);
  const latest = useRef(fn);
  latest.current = fn;
  const run = () => {
    if (latest.current) context.current.add(latest.current as unknown as gsap.ContextFunc, scope as unknown as Element);
  };

  // Runs first on mount. With dependencies, cleanup waits for unmount
  // (re-runs on dependency changes only add to the same context).
  const firstRun = useRef(true);
  useIso(() => {
    mounted.current = true;
    return deferCleanup ? teardown : undefined;
  }, []);

  useIso(() => {
    if (active.current) return;
    const el = scope?.current;
    if (!el || typeof IntersectionObserver === "undefined") {
      active.current = true;
      run();
      return;
    }
    const io = new IntersectionObserver(
      ([e]) => {
        if (!e.isIntersecting || active.current) return;
        io.disconnect();
        active.current = true;
        run();
      },
      // within a screen below, or anywhere above (already scrolled past)
      { rootMargin: "100000px 0px 100% 0px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useIso(() => {
    // A component mounting for the first time means the page change (if
    // any) has finished removing the old page.
    if (firstRun.current) {
      firstRun.current = false;
      leavingPage = false;
    }
    if (active.current) run();
    if (!deferCleanup) return teardown;
  }, deps);

  return { context: context.current, contextSafe: contextSafe.current };
}

export { gsap, ScrollTrigger, SplitText };
