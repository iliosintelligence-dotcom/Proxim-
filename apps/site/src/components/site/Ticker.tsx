import { tickerItems } from "@/lib/data";
import { Photo } from "@/components/phone/Photo";
import { Marquee } from "@/components/motion/Marquee";

/**
 * The kinds of apps ads can run in, each with a small photo of the people
 * who use them. Moves with your scroll; pauses on hover.
 */
export function Ticker({
  label = "Your ads can run inside",
  items = tickerItems,
}: {
  label?: string;
  items?: { label: string; photo: string }[];
}) {
  return (
    <div className="border-t bg-paper" style={{ borderColor: "var(--line)" }}>
      <div
        className="relative mx-auto flex max-w-[1240px] items-center gap-6 overflow-hidden py-5 lg:border-x"
        style={{ borderColor: "var(--line)" }}
      >
        <p className="t-label relative z-10 hidden shrink-0 bg-paper pl-[var(--gutter)] pr-2 text-faint md:block">{label}</p>
        <Marquee className="flex-1 [mask-image:linear-gradient(to_right,transparent,black_8%,black_92%,transparent)]">
          {items.map((t) => (
            <span
              key={t.label}
              className="flex items-center gap-2.5 whitespace-nowrap rounded-full border border-[var(--line)] bg-white py-1 pl-1 pr-4 shadow-[0_1px_2px_rgb(0_0_0/0.04)]"
            >
              <span className="block size-8 overflow-hidden rounded-full">
                <Photo id={t.photo} w={64} h={64} />
              </span>
              <span className="text-[0.95rem] font-medium tracking-[-0.01em] text-ink-2">{t.label}</span>
            </span>
          ))}
        </Marquee>
      </div>
    </div>
  );
}
