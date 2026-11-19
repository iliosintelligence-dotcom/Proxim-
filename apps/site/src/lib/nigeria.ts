/*
  A simplified outline of Nigeria (lon, lat), accurate enough for a
  stylised dot-matrix map. Not for navigation.
*/
export const outline: [number, number][] = [
  [2.69, 6.36], [2.72, 7.05], [2.75, 7.87], [2.95, 8.6], [3.13, 9.05], [3.37, 9.62],
  [3.6, 10.28], [3.6, 10.8], [3.69, 11.42], [3.6, 11.69], [3.93, 12.4], [4.1, 13.0],
  [4.13, 13.47], [4.9, 13.75], [5.52, 13.89], [6.27, 13.67], [6.82, 13.12], [7.33, 13.1],
  [7.83, 13.33], [8.6, 13.02], [9.01, 12.82], [9.64, 12.82], [10.15, 13.27], [10.7, 13.25],
  [11.53, 13.33], [12.3, 13.08], [12.87, 13.5], [13.32, 13.56], [13.62, 13.72], [14.1, 13.05],
  [14.18, 12.47], [14.65, 11.57], [14.37, 11.07], [13.6, 10.6], [13.25, 10.0], [13.17, 9.6],
  [12.88, 9.0], [12.6, 8.6], [12.24, 8.36], [11.75, 7.4], [11.84, 7.05], [11.1, 6.6],
  [10.7, 7.0], [10.5, 6.9], [10.1, 6.9], [9.7, 6.5], [9.43, 6.33], [8.8, 5.1],
  [8.5, 4.77], [8.3, 4.55], [7.9, 4.45], [7.47, 4.42], [7.08, 4.46], [6.7, 4.33],
  [6.1, 4.27], [5.9, 4.36], [5.36, 5.0], [5.25, 5.4], [4.95, 5.8], [4.5, 6.25],
  [3.9, 6.42], [3.4, 6.4],
];

export const bounds = { minLon: 2.5, maxLon: 14.85, minLat: 4.1, maxLat: 14.05 };

function inside(lon: number, lat: number) {
  let hit = false;
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const [xi, yi] = outline[i];
    const [xj, yj] = outline[j];
    if (yi > lat !== yj > lat && lon < ((xj - xi) * (lat - yi)) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}

export const MAP_W = 600;
export const MAP_H = Math.round(
  (MAP_W * (bounds.maxLat - bounds.minLat)) / (bounds.maxLon - bounds.minLon),
);
export const PX_PER_DEG = MAP_W / (bounds.maxLon - bounds.minLon);
export const KM_PER_DEG = 111;

export const project = (lon: number, lat: number) => ({
  x: (lon - bounds.minLon) * PX_PER_DEG,
  y: (bounds.maxLat - lat) * PX_PER_DEG,
});

/** Grid of dots that fall inside the country, built once. */
export function buildDots(step = 0.17) {
  const dots: { lon: number; lat: number; x: number; y: number }[] = [];
  for (let lat = bounds.minLat; lat <= bounds.maxLat; lat += step) {
    for (let lon = bounds.minLon; lon <= bounds.maxLon; lon += step) {
      if (inside(lon, lat)) dots.push({ lon, lat, ...project(lon, lat) });
    }
  }
  return dots;
}

export const distanceKm = (aLon: number, aLat: number, bLon: number, bLat: number) => {
  const dx = (aLon - bLon) * Math.cos(((aLat + bLat) / 2) * (Math.PI / 180));
  const dy = aLat - bLat;
  return Math.sqrt(dx * dx + dy * dy) * KM_PER_DEG;
};

/** PLACEHOLDER reach estimate: dense core, thinner suburbs beyond 15 km. */
export function estimateReach(density: number, radiusKm: number) {
  const core = Math.min(radiusKm, 15);
  const people = density * Math.PI * core * core + density * 0.12 * Math.PI * Math.max(0, radiusKm ** 2 - 15 ** 2);
  return Math.min(people, 38_000_000);
}
