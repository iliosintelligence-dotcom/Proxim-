"use client";

import { useEffect, useState } from "react";
import { MapPin } from "lucide-react";
import { cities } from "@/lib/data";
import { Globe } from "@/components/advertisers/Globe";

/**
 * The globe for "Where we're starting": turned to Nigeria with the eight
 * launch cities lit, touring from one to the next on its own. Sits over
 * the market photograph, so the place and the people share one frame.
 */
export function CitiesGlobe() {
  const [active, setActive] = useState(0);

  useEffect(() => {
    const id = window.setInterval(() => setActive((i) => (i + 1) % cities.length), 3200);
    return () => window.clearInterval(id);
  }, []);

  const city = cities[active];

  return (
    <div data-reveal className="relative">
      <div className="relative mx-auto aspect-square w-full max-w-[32rem]">
        {/* A dark disc behind the globe so its dots read over the photo. */}
        <div className="absolute inset-[6%] rounded-full bg-black/55 shadow-[0_0_80px_20px_rgb(0_0_0/0.35)] backdrop-blur-sm" />
        <div className="absolute inset-0 overflow-hidden [mask-image:radial-gradient(circle_at_50%_50%,black_58%,transparent_72%)]">
          <div className="absolute left-1/2 top-1/2 w-[118%] -translate-x-1/2 -translate-y-1/2">
            <Globe cityId={city.id} radius={50} />
          </div>
        </div>
      </div>
      <p
        aria-live="polite"
        className="absolute bottom-4 left-1/2 inline-flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-full border border-white/15 bg-black/40 px-4 py-2 text-[0.88rem] text-white backdrop-blur-md"
      >
        <MapPin className="size-4 text-[#5c9bff]" />
        Now showing <span className="font-medium">{city.name}</span>
        <span className="text-white/50">· 8 cities live</span>
      </p>
    </div>
  );
}
