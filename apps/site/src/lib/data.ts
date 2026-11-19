/*
  Content and sample numbers for the landing pages.

  Every price and rate in this file is a PLACEHOLDER until real pricing is
  set. They all live here so they can be changed in one place.
*/

export type Tint = "sun" | "lime" | "peach" | "sky" | "candy" | "lilac";

export const tintBg: Record<Tint, string> = {
  sun: "bg-ad-sun",
  lime: "bg-ad-lime",
  peach: "bg-ad-peach",
  sky: "bg-ad-sky",
  candy: "bg-ad-candy",
  lilac: "bg-ad-lilac",
};

/** A self-hosted Unsplash photo (free Unsplash licence), see /public/photos. */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const photo = (id: string, _w?: number, _h?: number) => `/photos/${id}.jpg`;

/** Each advertiser has its own typeface, like a real brand would. */
export type BrandFont = "serif" | "condensed" | "chunky" | "mono" | "wide" | "italic";

export type NicheId = "faith" | "fitness" | "food" | "education" | "music" | "fashion";

export type Niche = {
  id: NicheId;
  label: string;
  /** The kind of business that advertises in this niche. */
  kind: string;
  /** Pixel glyph shown wherever the niche is listed. */
  icon: "scripture" | "dumbbell" | "bowl" | "pencil" | "note" | "dress";
  hosts: string;
  why: string;
  app: { name: string };
  ad: {
    brand: string;
    initials: string;
    place: string;
    title: string;
    body: string;
    cta: string;
    tint: Tint;
    distance: string;
    photo: string;
    font: BrandFont;
  };
};

// All businesses and apps below are fictional examples.
export const niches: Niche[] = [
  {
    id: "faith",
    kind: "Church",
    icon: "scripture",
    label: "Faith",
    hosts: "Bible, Qur’an and devotional apps",
    why: "Someone reading scripture this morning is open to an invitation this Sunday.",
    app: { name: "Daily Word" },
    ad: {
      brand: "Grace Assembly",
      initials: "GA",
      place: "Lekki, Lagos",
      title: "Sunday service, 8 AM",
      body: "Come as you are. Free parking at the gate.",
      cta: "Get directions",
      tint: "sun",
      distance: "1.4 km",
      photo: "1609532587710-b00027c0594b",
      font: "serif",
    },
  },
  {
    id: "fitness",
    kind: "Gym",
    icon: "dumbbell",
    label: "Fitness",
    hosts: "Step counters, workout and running apps",
    why: "Someone who just logged a workout is the easiest person to sell a better gym to.",
    app: { name: "Stride" },
    ad: {
      brand: "FitHub Yaba",
      initials: "FH",
      place: "Yaba, Lagos",
      title: "Your first week is free",
      body: "Show this at the front desk to start.",
      cta: "Claim pass",
      tint: "lime",
      distance: "0.8 km",
      photo: "1597347343908-2937e7dcc560",
      font: "condensed",
    },
  },
  {
    id: "food",
    kind: "Kitchen",
    icon: "bowl",
    label: "Food",
    hosts: "Recipe, grocery and meal-planning apps",
    why: "Someone planning tonight’s menu is already deciding where the food comes from.",
    app: { name: "Pot & Pepper" },
    ad: {
      brand: "Mama Titi’s Kitchen",
      initials: "MT",
      place: "Surulere, Lagos",
      title: "Small chops for your party",
      body: "Trays from ₦15,000, delivered hot.",
      cta: "Order now",
      tint: "peach",
      distance: "2.1 km",
      photo: "1698827623494-c4e1196ba0ca",
      font: "chunky",
    },
  },
  {
    id: "education",
    kind: "Tutorial centre",
    icon: "pencil",
    label: "Education",
    hosts: "Exam prep, study and school apps",
    why: "A student practising past questions at midnight wants help passing.",
    app: { name: "Exam Ready" },
    ad: {
      brand: "Bright Minds Tutorial",
      initials: "BM",
      place: "Bodija, Ibadan",
      title: "JAMB weekend crash class",
      body: "Small groups. Past questions included.",
      cta: "Reserve a seat",
      tint: "sky",
      distance: "3.0 km",
      photo: "1744809482817-9a9d4fc280af",
      font: "mono",
    },
  },
  {
    id: "music",
    kind: "Lounge",
    icon: "note",
    label: "Music",
    hosts: "Streaming, radio and podcast apps",
    why: "Someone with Afrobeats on repeat is looking for somewhere to hear it live.",
    app: { name: "Groove" },
    ad: {
      brand: "Terra Lounge",
      initials: "TL",
      place: "Wuse 2, Abuja",
      title: "Afrobeats night, Friday",
      body: "Live band from 9 PM. Tables going fast.",
      cta: "Get tickets",
      tint: "candy",
      distance: "1.1 km",
      photo: "1565035010268-a3816f98589a",
      font: "wide",
    },
  },
  {
    id: "fashion",
    kind: "Boutique",
    icon: "dress",
    label: "Fashion",
    hosts: "Style, shopping and lookbook apps",
    why: "Someone saving outfits is one tap from buying the next one.",
    app: { name: "Lookbook" },
    ad: {
      brand: "Ankara & Co.",
      initials: "A&",
      place: "GRA, Port Harcourt",
      title: "The new ankara drop is in",
      body: "Hand-finished wax prints. Sizes 8 to 20.",
      cta: "Shop the drop",
      tint: "lilac",
      distance: "2.6 km",
      photo: "1687422809069-0fa3546b8471",
      font: "italic",
    },
  },
];

export const nicheById = (id: NicheId) => niches.find((n) => n.id === id)!;

/** App categories for the ticker, each with a photo of who uses them. */
export const tickerItems: { label: string; photo: string }[] = [
  { label: "Bible apps", photo: "1609532587710-b00027c0594b" },
  { label: "Step counters", photo: "1649888216899-047093431441" },
  { label: "Recipe apps", photo: "1666190092689-e3968aa0c32c" },
  { label: "Exam prep", photo: "1744809482817-9a9d4fc280af" },
  { label: "Music streaming", photo: "1565035010268-a3816f98589a" },
  { label: "Fashion", photo: "1687422809069-0fa3546b8471" },
  { label: "Qur’an apps", photo: "1665849863678-fc03a642a9aa" },
  { label: "Workout apps", photo: "1597347343908-2937e7dcc560" },
  { label: "Grocery apps", photo: "1698827623494-c4e1196ba0ca" },
  { label: "Study apps", photo: "1744809482817-9a9d4fc280af" },
  { label: "Podcasts", photo: "1643904524951-2a3a58856745" },
  { label: "Style boards", photo: "1625646741211-711bdd65c570" },
];

export type City = {
  id: string;
  name: string;
  state: string;
  lon: number;
  lat: number;
  /** Placeholder: rough smartphone users per km² near the centre. */
  density: number;
};

export const cities: City[] = [
  { id: "lagos", name: "Lagos", state: "Lagos", lon: 3.38, lat: 6.52, density: 5200 },
  { id: "ibadan", name: "Ibadan", state: "Oyo", lon: 3.9, lat: 7.38, density: 1400 },
  { id: "abuja", name: "Abuja", state: "FCT", lon: 7.49, lat: 9.06, density: 1500 },
  { id: "kano", name: "Kano", state: "Kano", lon: 8.52, lat: 12.0, density: 2100 },
  { id: "ph", name: "Port Harcourt", state: "Rivers", lon: 7.01, lat: 4.82, density: 1900 },
  { id: "enugu", name: "Enugu", state: "Enugu", lon: 7.5, lat: 6.45, density: 1100 },
  { id: "kaduna", name: "Kaduna", state: "Kaduna", lon: 7.44, lat: 10.52, density: 900 },
  { id: "benin", name: "Benin City", state: "Edo", lon: 5.62, lat: 6.34, density: 1000 },
];

export const radiusSteps = [2, 5, 10, 25, 50, 100, 200];

/** PLACEHOLDER pricing, in naira per 1,000 impressions. */
export const advertiserCpm = { banner: 1200, notification: 3500 } as const;
export type Format = keyof typeof advertiserCpm;

export const impressionSteps = [
  5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000,
];

/** PLACEHOLDER publisher earnings, in naira per 1,000 impressions served. */
export const publisherEcpm = { banner: 600, notification: 1800 } as const;
/** PLACEHOLDER share of ad requests that get filled. */
export const fillRate = 0.7;

/** PLACEHOLDER plans for the pricing page. */
export const plans = [
  {
    id: "payg",
    name: "Pay as you go",
    price: "₦0",
    per: "a month",
    pitch: "For a single business trying Proxim for the first time.",
    features: [
      "Banners from ₦1,200 per 1,000 views",
      "Notifications from ₦3,500 per 1,000 views",
      "Campaigns from ₦6,000",
      "Live reporting",
    ],
    cta: "Start a campaign",
    featured: false,
  },
  {
    id: "growth",
    name: "Growth",
    price: "₦50,000",
    per: "a month, as ad credit",
    pitch: "For businesses that advertise every month and want more for it.",
    features: [
      "15% lower rates on every view",
      "Up to 3 locations or brands",
      "Ads reviewed within 2 hours",
      "Monthly performance report",
    ],
    cta: "Choose Growth",
    featured: true,
  },
  {
    id: "agency",
    name: "Agencies and brands",
    price: "Custom",
    per: "volume pricing",
    pitch: "For agencies and brands running many campaigns across states.",
    features: [
      "Unlimited clients and locations",
      "One invoice for every campaign",
      "Dedicated account manager",
      "API access",
    ],
    cta: "Talk to sales",
    featured: false,
  },
];

/** PLACEHOLDER share of ad revenue paid to app owners. */
export const publisherShare = 0.7;

export const pricingFaqs = [
  {
    q: "What counts as a view?",
    a: "A view is counted when your ad is actually on someone’s screen, not when it loads in the background. You are never charged for an ad nobody could see.",
  },
  {
    q: "Is there a minimum spend or contract?",
    a: "No contract. The smallest campaign is 5,000 views, and you can stop a campaign at any time. Unused views are not charged.",
  },
  {
    q: "How do I pay?",
    a: "Pay in naira by card or bank transfer. Growth and agency plans can be invoiced monthly.",
  },
  {
    q: "How fast does my ad go live?",
    a: "Most ads are reviewed and live within a few hours. Growth plans are reviewed within 2 hours.",
  },
  {
    q: "Can I change the radius or budget after launch?",
    a: "Yes. Change the area, dates or number of views while the campaign runs. The price updates before you confirm.",
  },
  {
    q: "What does it cost to show Proxim ads in my app?",
    a: "Nothing. App owners join free and keep a share of every view. See the app owners page for details.",
  },
];

/** PLACEHOLDER contact details. Replace before launch. */
export const contact = {
  sales: "sales@proxim.ng",
  partners: "apps@proxim.ng",
  support: "help@proxim.ng",
  city: "Lagos, Nigeria",
  whatsapp: "#",
};

/** Photos used inside the host apps (all free Unsplash licence). */
export const appPhotos = {
  verse: "1665849863678-fc03a642a9aa",
  sunrise: "1707730088436-0e55e78843d8",
  jollof: "1666190092689-e3968aa0c32c",
  workout: "1649888216899-047093431441",
  album: "1643904524951-2a3a58856745",
  looks: ["1625646741211-711bdd65c570", "1760907949889-eb62b7fd9f75", "1790862303813-8fdee084284d", "1773398972684-2ba88aa89499"],
  student: "1620829813573-7c9e1877706f",
};
