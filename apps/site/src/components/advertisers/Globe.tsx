"use client";

import { useEffect, useRef } from "react";
import createGlobe, { type Arc, type Globe as CobeGlobe, type Marker } from "cobe";
import { cities, type City } from "@/lib/data";
import { prefersReducedMotion } from "@/lib/gsap";

/*
  A dotted WebGL globe (cobe), turned to face Nigeria. The chosen city
  glows blue, phones inside the radius light up around it, and short arcs
  run from the business to those phones: the ad, travelling to the people
  who can actually walk in. Drag to spin it; it eases back on release.
*/

const toAngles = (lat: number, lon: number) => ({
  phi: Math.PI - ((lon * Math.PI) / 180 - Math.PI / 2),
  theta: (lat * Math.PI) / 180,
});

/** Deterministic scatter of "phones" around a city, inside the radius. */
function phonesAround(city: City, radiusKm: number, seed: number) {
  // Street-level radii would all sit on one dot at globe scale, so the
  // spread has a visible minimum; the numbers beside the globe are exact.
  const spread = Math.max(radiusKm, 28) / 111;
  const pts: [number, number][] = [];
  let s = seed * 9301 + 49297;
  const rand = () => ((s = (s * 9301 + 49297) % 233280) / 233280);
  const count = Math.round(10 + Math.min(radiusKm, 200) / 8);
  for (let i = 0; i < count; i++) {
    const a = rand() * Math.PI * 2;
    const r = Math.sqrt(rand()) * spread;
    pts.push([city.lat + r * Math.sin(a), city.lon + (r * Math.cos(a)) / Math.cos((city.lat * Math.PI) / 180)]);
  }
  return pts;
}

function build(city: City, radius: number) {
  const idx = cities.indexOf(city);
  const phones = phonesAround(city, radius, idx + 1);
  const markers: Marker[] = [
    ...cities
      .filter((c) => c.id !== city.id)
      .map((c) => ({ location: [c.lat, c.lon] as [number, number], size: 0.022, color: [0.85, 0.87, 0.95] as [number, number, number] })),
    ...phones.map((p) => ({ location: p, size: 0.011 })),
    { location: [city.lat, city.lon], size: 0.055 },
  ];
  const arcs: Arc[] = phones.slice(0, 7).map((p) => ({ from: [city.lat, city.lon], to: p }));
  return { markers, arcs };
}

export function Globe({ cityId, radius, className = "" }: { cityId: string; radius: number; className?: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const globeRef = useRef<CobeGlobe | null>(null);
  const target = useRef(toAngles(9, 8));
  const current = useRef({ ...target.current });
  const drag = useRef<{ x: number; y: number } | null>(null);
  const dragOffset = useRef({ phi: 0, theta: 0 });

  // Create once.
  useEffect(() => {
    const canvas = canvasRef.current!;
    const reduce = prefersReducedMotion();
    const city = cities.find((c) => c.id === cityId)!;
    target.current = toAngles(city.lat - 1, city.lon);
    current.current = { ...target.current };
    let width = canvas.offsetWidth;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    const { markers, arcs } = build(city, radius);
    const globe = createGlobe(canvas, {
      devicePixelRatio: dpr,
      width: width * dpr,
      height: width * dpr,
      phi: current.current.phi,
      theta: current.current.theta,
      dark: 1,
      diffuse: 1.6,
      mapSamples: 22000,
      mapBrightness: 5.5,
      mapBaseBrightness: 0.04,
      baseColor: [0.26, 0.26, 0.28],
      markerColor: [0.0, 0.42, 1.0],
      glowColor: [0.07, 0.11, 0.24],
      arcColor: [0.25, 0.55, 1.0],
      arcWidth: 0.6,
      arcHeight: 0.12,
      markerElevation: 0.01,
      markers,
      arcs,
      scale: 1.5,
    });
    globeRef.current = globe;

    let raf = 0;
    let visible = true;
    let t = 0;
    const tick = () => {
      t += 0.004;
      const drift = reduce || drag.current ? 0 : Math.sin(t) * 0.08;
      const c = current.current;
      c.phi += (target.current.phi + dragOffset.current.phi + drift - c.phi) * 0.06;
      c.theta += (target.current.theta + dragOffset.current.theta - c.theta) * 0.06;
      if (!drag.current) {
        dragOffset.current.phi *= 0.94;
        dragOffset.current.theta *= 0.94;
      }
      globe.update({ phi: c.phi, theta: c.theta, width: width * dpr, height: width * dpr });
      if (visible) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    const io = new IntersectionObserver(([e]) => {
      visible = e.isIntersecting;
      cancelAnimationFrame(raf);
      if (visible) raf = requestAnimationFrame(tick);
    });
    io.observe(canvas);
    const ro = new ResizeObserver(() => (width = canvas.offsetWidth));
    ro.observe(canvas);
    requestAnimationFrame(() => canvas.style.setProperty("opacity", "1"));

    return () => {
      cancelAnimationFrame(raf);
      io.disconnect();
      ro.disconnect();
      globe.destroy();
      globeRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Turn to the chosen city and redraw its radius.
  useEffect(() => {
    const city = cities.find((c) => c.id === cityId)!;
    target.current = toAngles(city.lat - 1, city.lon);
    globeRef.current?.update(build(city, radius));
  }, [cityId, radius]);

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      onPointerDown={(e) => {
        drag.current = { x: e.clientX, y: e.clientY };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        dragOffset.current.phi += (e.clientX - drag.current.x) / 220;
        dragOffset.current.theta = Math.max(-0.6, Math.min(0.6, dragOffset.current.theta + (e.clientY - drag.current.y) / 400));
        drag.current = { x: e.clientX, y: e.clientY };
      }}
      onPointerUp={() => (drag.current = null)}
      onPointerCancel={() => (drag.current = null)}
      className={`aspect-square w-full cursor-grab touch-pan-y opacity-0 transition-opacity duration-1000 active:cursor-grabbing ${className}`}
    />
  );
}
