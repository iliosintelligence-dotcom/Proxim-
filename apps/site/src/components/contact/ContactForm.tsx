"use client";

import { MessageCircle } from "lucide-react";
import { useRef, useState, type FormEvent } from "react";
import { gsap, useGSAP, prefersReducedMotion } from "@/lib/gsap";
import { AppIcon } from "@/components/site/AppIcon";
import { PixelArrow } from "@/components/site/Button";

type Role = "advertiser" | "app" | "other";

const roles: { id: Role; label: string }[] = [
  { id: "advertiser", label: "I want to advertise" },
  { id: "app", label: "I own an app" },
  { id: "other", label: "Something else" },
];

const field =
  "mt-2 h-12 w-full rounded-xl border border-[var(--line-strong)] bg-white px-4 text-[0.95rem] text-ink placeholder:text-faint transition-[border-color,box-shadow] duration-200 focus:border-ink focus:outline-none focus:ring-4 focus:ring-blue/15 aria-[invalid=true]:border-red-500";

/**
 * The form changes with who's asking: advertisers get a budget question,
 * app owners get an audience question. Fewer, better questions.
 *
 * NOTE: not connected to a backend yet. Submitting validates and shows
 * the sent state only.
 */
export function ContactForm() {
  const [role, setRole] = useState<Role>("advertiser");
  const [sent, setSent] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (prefersReducedMotion()) return;
      gsap.fromTo("[data-role-field]", { autoAlpha: 0, y: 8 }, { autoAlpha: 1, y: 0, duration: 0.5, ease: "expo.out" });
    },
    { scope: root, dependencies: [role], revertOnUpdate: true },
  );

  useGSAP(
    () => {
      if (!sent || prefersReducedMotion()) return;
      gsap.fromTo(
        "[data-sent] [data-flick]",
        { opacity: 0 },
        { keyframes: { opacity: [0, 1, 0.2, 1], easeEach: "steps(1)" }, duration: 0.4, ease: "none", stagger: 0.08 },
      );
    },
    { scope: root, dependencies: [sent] },
  );

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const data = new FormData(e.currentTarget);
    const next: Record<string, string> = {};
    if (!String(data.get("name") || "").trim()) next.name = "Tell us your name.";
    const email = String(data.get("email") || "").trim();
    if (!/^\S+@\S+\.\S+$/.test(email)) next.email = "Enter an email address we can reply to.";
    if (!String(data.get("message") || "").trim()) next.message = "Add a short message.";
    setErrors(next);
    if (Object.keys(next).length === 0) setSent(true);
  };

  const err = (k: string) =>
    errors[k] ? (
      <p id={`${k}-error`} className="mt-1.5 text-[0.8rem] text-red-600">
        {errors[k]}
      </p>
    ) : null;

  if (sent) {
    return (
      <div ref={root}>
        <div data-sent className="flex min-h-[28rem] flex-col items-start justify-center rounded-3xl border border-[var(--line)] bg-white p-8 sm:p-10">
          <span data-flick>
            <AppIcon icon={MessageCircle} tone="green" size="lg" />
          </span>
          <p data-flick className="t-section mt-8">
            Message sent.
          </p>
          <p data-flick className="mt-3 max-w-[28rem] text-[1rem] leading-relaxed text-muted-foreground">
            Thanks. Someone from the team will reply within one working day.
          </p>
          <button
            type="button"
            onClick={() => setSent(false)}
            className="group mt-8 inline-flex items-center gap-2 text-[0.92rem] font-medium text-ink"
          >
            Send another message <PixelArrow />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div ref={root}>
      <form
        noValidate
        onSubmit={submit}
        className="rounded-3xl border border-[var(--line)] bg-white p-6 shadow-[0_30px_60px_-36px_rgb(28_25_23/0.35)] sm:p-9"
      >
        <fieldset>
          <legend className="t-label text-faint">You are</legend>
          <div role="radiogroup" className="mt-3 grid gap-2 sm:grid-cols-3">
            {roles.map((r) => (
              <button
                key={r.id}
                type="button"
                role="radio"
                aria-checked={role === r.id}
                onClick={() => setRole(r.id)}
                className={`rounded-xl border px-3 py-3 text-[0.88rem] font-medium transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue ${
                  role === r.id ? "border-ink bg-ink text-white" : "border-[var(--line-strong)] text-ink-2 hover:border-ink/30"
                }`}
              >
                {r.label}
              </button>
            ))}
          </div>
        </fieldset>

        <div className="mt-7 grid gap-5 sm:grid-cols-2">
          <label className="block">
            <span className="text-[0.88rem] font-medium">Full name</span>
            <input name="name" autoComplete="name" className={field} aria-invalid={!!errors.name} aria-describedby={errors.name ? "name-error" : undefined} />
            {err("name")}
          </label>
          <label className="block">
            <span className="text-[0.88rem] font-medium">Work email</span>
            <input name="email" type="email" autoComplete="email" className={field} aria-invalid={!!errors.email} aria-describedby={errors.email ? "email-error" : undefined} />
            {err("email")}
          </label>
          <label className="block">
            <span className="text-[0.88rem] font-medium">
              Phone <span className="font-normal text-faint">(optional)</span>
            </span>
            <input name="phone" type="tel" autoComplete="tel" placeholder="080..." className={field} />
          </label>
          <label className="block">
            <span className="text-[0.88rem] font-medium">{role === "app" ? "App name" : "Business name"}</span>
            <input name="company" autoComplete="organization" className={field} />
          </label>

          {role !== "other" && (
            <label data-role-field className="block sm:col-span-2">
              <span className="text-[0.88rem] font-medium">
                {role === "advertiser" ? "Monthly ad budget" : "Daily active users"}
              </span>
              <span className="relative block">
                <select name="size" className={`${field} appearance-none pr-10`} defaultValue="">
                  <option value="" disabled>
                    Choose one
                  </option>
                  {(role === "advertiser"
                    ? ["Under ₦50,000", "₦50,000 to ₦250,000", "₦250,000 to ₦1,000,000", "Over ₦1,000,000"]
                    : ["Under 5,000", "5,000 to 50,000", "50,000 to 250,000", "Over 250,000"]
                  ).map((o) => (
                    <option key={o}>{o}</option>
                  ))}
                </select>
                <svg viewBox="0 0 7 4" className="pointer-events-none absolute right-4 top-[calc(50%+0.25rem)] h-1 -translate-y-1/2 text-muted-foreground" shapeRendering="crispEdges" aria-hidden="true">
                  <path d="M0 0h1v1H0zM1 1h1v1H1zM2 2h1v1H2zM3 3h1v1H3zM4 2h1v1H4zM5 1h1v1H5zM6 0h1v1H6z" fill="currentColor" />
                </svg>
              </span>
            </label>
          )}

          <label className="block sm:col-span-2">
            <span className="text-[0.88rem] font-medium">How can we help?</span>
            <textarea
              name="message"
              rows={5}
              className={`${field} h-auto resize-y py-3`}
              placeholder={role === "app" ? "Tell us about your app and your users." : "Tell us what you want to promote and where."}
              aria-invalid={!!errors.message}
              aria-describedby={errors.message ? "message-error" : undefined}
            />
            {err("message")}
          </label>
        </div>

        <div className="mt-7 flex flex-col-reverse items-start justify-between gap-4 sm:flex-row sm:items-center">
          <p className="text-[0.8rem] text-faint">We reply within one working day.</p>
          <button
            type="submit"
            className="group inline-flex h-12 items-center gap-2 rounded-full bg-blue px-6 text-[0.95rem] font-medium text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.18),0_1px_2px_rgb(0_40_120/0.25)] transition-[background-color,transform] duration-150 hover:bg-blue-deep active:scale-[0.98] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue"
          >
            Send message <PixelArrow />
          </button>
        </div>
      </form>
    </div>
  );
}
