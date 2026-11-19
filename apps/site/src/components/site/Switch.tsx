"use client";

/** On/off switch shared across pages. */
export function Switch({
  on,
  onChange,
  label,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      className={`relative h-7 w-12 shrink-0 rounded-full transition-colors duration-300 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${on ? "bg-ink" : "bg-stone-2"}`}
    >
      <span
        className={`absolute left-1 top-1 size-5 rounded-full bg-white shadow-sm transition-transform duration-300 ease-[var(--ease-out-quint)] ${on ? "translate-x-5" : ""}`}
      />
    </button>
  );
}
