"use client";

import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import type { Format, NicheId } from "@/lib/data";

type Campaign = {
  niche: NicheId;
  format: Format;
  city: string;
  radius: number;
  days: number;
  impressions: number;
};

type Ctx = Campaign & { set: <K extends keyof Campaign>(k: K, v: Campaign[K]) => void };

const CampaignCtx = createContext<Ctx | null>(null);

/**
 * The advertisers page is one campaign being built as you scroll. Each
 * section sets one part of it, and the ticket at the end adds it all up,
 * so by the time someone reaches the price they've already made the ad.
 */
export function CampaignProvider({ children }: { children: ReactNode }) {
  const [c, setC] = useState<Campaign>({
    niche: "faith",
    format: "banner",
    city: "lagos",
    radius: 10,
    days: 14,
    impressions: 50_000,
  });
  const value = useMemo<Ctx>(
    () => ({ ...c, set: (k, v) => setC((prev) => ({ ...prev, [k]: v })) }),
    [c],
  );
  return <CampaignCtx.Provider value={value}>{children}</CampaignCtx.Provider>;
}

export function useCampaign() {
  const ctx = useContext(CampaignCtx);
  if (!ctx) throw new Error("useCampaign must be used inside CampaignProvider");
  return ctx;
}
