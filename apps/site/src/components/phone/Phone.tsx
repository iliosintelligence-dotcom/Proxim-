import type { ReactNode } from "react";

/*
  iPhone 15 at true proportions. Inside the screen, Tailwind's spacing
  unit is one iOS point (see .phone-ui), so apps are built straight from
  Apple's specs: p-16 is 16pt, fs-17 is 17pt body, h-54 is the status
  bar, h-83 is the tab bar. The whole phone scales to any width.
*/

export function Phone({
  children,
  className = "",
  screenClassName = "bg-white",
  label,
}: {
  children: ReactNode;
  className?: string;
  screenClassName?: string;
  label?: string;
}) {
  return (
    <div role={label ? "img" : undefined} aria-label={label} className={`relative ${className}`}>
      {/* action, volume and power buttons */}
      <span aria-hidden className="absolute -left-[0.9%] top-[17%] h-[3.6%] w-[1.1%] rounded-l-sm bg-[#3a3835]" />
      <span aria-hidden className="absolute -left-[0.9%] top-[24%] h-[7%] w-[1.1%] rounded-l-sm bg-[#3a3835]" />
      <span aria-hidden className="absolute -left-[0.9%] top-[32.5%] h-[7%] w-[1.1%] rounded-l-sm bg-[#3a3835]" />
      <span aria-hidden className="absolute -right-[0.9%] top-[27%] h-[11%] w-[1.1%] rounded-r-sm bg-[#3a3835]" />
      <div className="relative rounded-[15.5%/7.3%] bg-[linear-gradient(145deg,#4a4844,#1b1a18_38%,#2c2b28_70%,#121110)] p-[2.4%] shadow-[0_0_0_1px_rgb(0_0_0/0.5),0_2px_4px_rgb(0_0_0/0.12),0_40px_80px_-30px_rgb(28_25_23/0.45),0_24px_40px_-24px_rgb(28_25_23/0.35)]">
        <div className="rounded-[13.8%/6.5%] bg-black p-[1%]">
          <div className={`phone-screen relative aspect-[393/852] w-full overflow-hidden rounded-[12.8%/5.9%] ${screenClassName}`}>
            <div className="phone-ui absolute inset-0">{children}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

/** iOS status bar: 54pt tall, time left, Dynamic Island centred, system icons right. */
export function StatusBar({ time = "9:41", light = false }: { time?: string; light?: boolean }) {
  return (
    <div className={`relative z-30 flex h-54 items-center justify-between pl-34 pr-30 pt-6 ${light ? "text-white" : "text-black"}`}>
      <span className="w-54 text-center fs-17 font-semibold tracking-[-0.02em]">{time}</span>
      <span aria-hidden className="absolute left-1/2 top-11 h-37 w-126 -translate-x-1/2 rounded-full bg-black" />
      <span aria-hidden className="flex items-center gap-6">
        <svg viewBox="0 0 18 12" className="h-12 fill-current">
          <rect x="0" y="7.5" width="3" height="4.5" rx="0.8" />
          <rect x="5" y="5" width="3" height="7" rx="0.8" />
          <rect x="10" y="2.5" width="3" height="9.5" rx="0.8" />
          <rect x="15" y="0" width="3" height="12" rx="0.8" />
        </svg>
        <svg viewBox="0 0 16 12" className="h-12 fill-current">
          <path d="M8 2.6c2.2 0 4.2.8 5.7 2.2l1.2-1.3A10 10 0 0 0 8 .8 10 10 0 0 0 1.1 3.5l1.2 1.3A8.2 8.2 0 0 1 8 2.6Zm0 3.4c1.3 0 2.5.5 3.4 1.3l1.2-1.3A6.6 6.6 0 0 0 8 4.2c-1.8 0-3.4.7-4.6 1.8l1.2 1.3C5.5 6.5 6.7 6 8 6Zm0 3.3c.5 0 .9.2 1.2.5L8 11.2 6.8 9.8c.3-.3.7-.5 1.2-.5Z" />
        </svg>
        <svg viewBox="0 0 27 13" className="h-13">
          <rect x="0.5" y="0.5" width="23" height="12" rx="3.8" fill="none" stroke="currentColor" opacity="0.35" />
          <rect x="2" y="2" width="20" height="9" rx="2.5" fill="currentColor" />
          <path d="M25 4.5v4c.8-.3 1.3-1.1 1.3-2s-.5-1.7-1.3-2Z" fill="currentColor" opacity="0.4" />
        </svg>
      </span>
    </div>
  );
}

export function HomeIndicator({ light = false }: { light?: boolean }) {
  return (
    <span
      aria-hidden
      className={`absolute bottom-8 left-1/2 z-40 h-5 w-134 -translate-x-1/2 rounded-full ${light ? "bg-white" : "bg-black"}`}
    />
  );
}
