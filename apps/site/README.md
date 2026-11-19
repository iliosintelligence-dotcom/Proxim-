# Proxim marketing site

The landing pages for advertisers and app owners: home, app owners, features, pricing, about and contact.

Built with Next.js 15 (App Router), React 19, Tailwind CSS 4, shadcn/ui, GSAP and Lenis. Typography is Momo Trust Display (headlines) and Inter (everything else).

## Run it

```
cd apps/site
npm install
npm run dev        # http://localhost:3000
npm run build      # static export to apps/site/out
```

## How it is served

`next.config.ts` sets `output: "export"`, so the build is plain static HTML, CSS and JavaScript in `out/`. The root `vercel.json` installs and builds this folder and serves `apps/site/out` with clean URLs and a Content Security Policy that only allows the site's own origin.

Everything is self-hosted, so the pages make no requests to other domains:

- Photos: `public/photos/` (Unsplash License; file names are Unsplash photo IDs)
- Momo Trust Display: `public/fonts/` (SIL Open Font License 1.1)
- Inter: bundled at build time by `next/font`

## Where things live

| Path | What |
|---|---|
| `src/app/` | Pages: `/`, `/publishers`, `/features`, `/pricing`, `/about`, `/contact` |
| `src/components/site/` | Shared pieces: nav, footer, finale, tickers, buttons, icons, cookie notice |
| `src/components/phone/` | iPhone mockups, host app screens and the ad formats |
| `src/components/mascot/` | Pip, the mascot and logo |
| `src/lib/data.ts` | All sample content and placeholder prices (replace before launch) |
| `public/console/` | The earlier business console prototype pages, kept as they were |

Sample businesses, apps and figures on the site are fictional placeholders.
