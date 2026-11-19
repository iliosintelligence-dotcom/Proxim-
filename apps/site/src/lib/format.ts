export const naira = (n: number) => "₦" + Math.round(n).toLocaleString("en-NG");

export const compact = (n: number) => {
  if (n >= 1_000_000) return `${+(n / 1_000_000).toFixed(n % 1_000_000 ? 1 : 0)}M`;
  if (n >= 1_000) return `${+(n / 1_000).toFixed(n % 1_000 ? 1 : 0)}K`;
  return `${n}`;
};

export const commas = (n: number) => Math.round(n).toLocaleString("en-NG");
