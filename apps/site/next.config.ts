import type { NextConfig } from "next";

// Exported as plain static files, so the repo's Vercel setup can serve
// apps/site directly without a server.
const nextConfig: NextConfig = {
  output: "export",
  images: { unoptimized: true },
};

export default nextConfig;
