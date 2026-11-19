import type { Metadata, Viewport } from "next";
import { Inter } from "next/font/google";
import { MotionDirector } from "@/components/motion/MotionDirector";
import { Nav } from "@/components/site/Nav";
import { Footer } from "@/components/site/Footer";
import { PipGuide } from "@/components/mascot/PipGuide";
import { CookieNotice } from "@/components/site/CookieNotice";
import "./globals.css";

// Ledger's pairing: Momo Trust Display for headlines and big numbers
// (self-hosted in /public/fonts, see globals.css), Inter for everything
// else (bundled into the build by next/font). No font requests leave the site.
const sans = Inter({
  subsets: ["latin"],
  variable: "--nf-inter",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Proxim: Ads inside the apps your customers already use",
  description:
    "Place banner and notification ads inside apps that match your niche, shown only to people near you. Built for businesses across Nigeria.",
};

export const viewport: Viewport = {
  themeColor: "#f3f3f3",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={sans.variable} suppressHydrationWarning>
      <head>
        {/* Marks JS as available before first paint so animated text
            starts hidden instead of flashing. */}
        <script
          dangerouslySetInnerHTML={{ __html: "document.documentElement.classList.add('js')" }}
        />
      </head>
      <body>
        <MotionDirector />
        <Nav />
        <main>{children}</main>
        <Footer />
        <PipGuide />
        <CookieNotice />
      </body>
    </html>
  );
}
