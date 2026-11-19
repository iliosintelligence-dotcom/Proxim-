import { MapPin } from "lucide-react";
import { Photo } from "@/components/phone/Photo";
import { Marquee } from "@/components/motion/Marquee";
import { LogoMark } from "./Logo";

/*
  Sample SME ad posters (after the "Imagine social commerce" reference):
  bold colour, the business name set large, and the owner's photo cut
  into a playful shape. All businesses are fictional examples.
*/

type Shape = "flower" | "wave" | "arch" | "blob";

type Poster = { name: string; place: string; photo: string; bg: string; fg: string; shape: Shape };

const posters: Poster[] = [
  { name: "Mama Titi’s Kitchen", place: "Surulere · 2.1 km", photo: "1698827623494-c4e1196ba0ca", bg: "#ff5b1f", fg: "#ffde50", shape: "flower" },
  { name: "Ankara & Co.", place: "GRA · 2.6 km", photo: "1687422809069-0fa3546b8471", bg: "#ffde50", fg: "#ff5b1f", shape: "wave" },
  { name: "Grace Assembly", place: "Lekki · 1.4 km", photo: "1609532587710-b00027c0594b", bg: "#f2f2f2", fg: "#8d8d8d", shape: "arch" },
  { name: "FitHub Yaba", place: "Yaba · 0.8 km", photo: "1597347343908-2937e7dcc560", bg: "#b6f24a", fg: "#0d0d0d", shape: "blob" },
  { name: "Glow Salon Ikeja", place: "Ikeja · 1.9 km", photo: "1702236240794-58dc4c6895e5", bg: "#2f3cff", fg: "#34d6f0", shape: "flower" },
  { name: "Corner Butchers", place: "Ajah · 0.6 km", photo: "1687422808248-f807f4ea2a2e", bg: "#34d6f0", fg: "#2f3cff", shape: "arch" },
  { name: "Bright Minds", place: "Bodija · 3.0 km", photo: "1744809482817-9a9d4fc280af", bg: "#ff8fd0", fg: "#2f3cff", shape: "wave" },
  { name: "Kola Pharmacy", place: "Wuse · 1.2 km", photo: "1576091358783-a212ec293ff3", bg: "#a8f0d4", fg: "#0b6b4a", shape: "blob" },
];

/** Shapes as objectBoundingBox clip paths, generated once. */
function polar(fn: (a: number) => number, steps = 120) {
  const pts: string[] = [];
  for (let i = 0; i <= steps; i++) {
    const a = (i / steps) * Math.PI * 2;
    const r = fn(a);
    pts.push(`${(0.5 + r * Math.cos(a)).toFixed(4)},${(0.5 + r * Math.sin(a)).toFixed(4)}`);
  }
  return `M${pts.join("L")}Z`;
}

const shapePaths: Record<Shape, string> = {
  flower: polar((a) => 0.43 + 0.06 * Math.cos(10 * a)),
  wave: polar((a) => 0.46 + 0.025 * Math.cos(18 * a)),
  blob: polar((a) => 0.44 + 0.04 * Math.sin(3 * a + 0.6) + 0.025 * Math.cos(5 * a)),
  arch: "M0.04,1 L0.04,0.46 C0.04,0.2 0.24,0.02 0.5,0.02 C0.76,0.02 0.96,0.2 0.96,0.46 L0.96,1 Z",
};

export function PosterShapes() {
  return (
    <svg width="0" height="0" className="absolute" aria-hidden="true">
      <defs>
        {(Object.keys(shapePaths) as Shape[]).map((k) => (
          <clipPath key={k} id={`shape-${k}`} clipPathUnits="objectBoundingBox">
            <path d={shapePaths[k]} />
          </clipPath>
        ))}
      </defs>
    </svg>
  );
}

function PosterCard({ p }: { p: Poster }) {
  return (
    <article
      className="group w-full rounded-3xl p-5 shadow-[0_24px_50px_-24px_rgb(0_0_0/0.45)] transition-transform duration-500 ease-[var(--ease-out-quint)] hover:-rotate-2 hover:scale-[1.03]"
      style={{ background: p.bg }}
    >
      <div className="flex items-start justify-between gap-3">
        <p className="font-display text-[1.65rem] leading-[1.02] tracking-[-0.03em]" style={{ color: p.fg }}>
          {p.name}
        </p>
        <span className="grid size-8 shrink-0 place-items-center rounded-full bg-white/85">
          <LogoMark className="size-4" />
        </span>
      </div>
      <div className="relative mt-4 aspect-square w-full transition-transform duration-700 ease-[var(--ease-out-quint)] group-hover:scale-[1.04]" style={{ clipPath: `url(#shape-${p.shape})` }}>
        <Photo id={p.photo} w={400} h={400} />
      </div>
      <p className="mt-3 inline-flex items-center gap-1 rounded-full bg-black/10 px-2.5 py-1 text-[0.72rem] font-medium" style={{ color: p.fg === "#8d8d8d" ? "#0d0d0d" : p.fg }}>
        <MapPin className="size-3" /> Sponsored · {p.place}
      </p>
    </article>
  );
}

/** Two columns of posters: the left one runs down, the right one runs up. */
export function AdPosterWall({ className = "" }: { className?: string }) {
  const a = posters.filter((_, i) => i % 2 === 0);
  const b = posters.filter((_, i) => i % 2 === 1);
  return (
    <div className={`relative grid grid-cols-2 gap-4 overflow-hidden [mask-image:linear-gradient(to_bottom,transparent,black_12%,black_88%,transparent)] ${className}`}>
      <PosterShapes />
      <div data-col="down" className="h-full">
        <Marquee vertical reverse seconds={38} gap="gap-4" className="h-full">
          {a.map((p) => (
            <PosterCard key={p.name} p={p} />
          ))}
        </Marquee>
      </div>
      <div data-col="up" className="h-full">
        <Marquee vertical seconds={46} gap="gap-4" className="h-full pt-24">
          {b.map((p) => (
            <PosterCard key={p.name} p={p} />
          ))}
        </Marquee>
      </div>
    </div>
  );
}
