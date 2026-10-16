//! H3 helpers (spec 4): cell lookup against the reference C library (vectors produced with
//! `h3` 4.5.0 for Python), grid disks, boundaries, edge distance and circle covers.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

mod common;

use common::{destination, pos};
use proptest::prelude::*;
use proxim_geo::frame_for;
use proxim_geo::h3::{
    MAX_EXPORT_RES, cell_boundary, cell_center, cell_of, cell_resolution, cover_circle,
    dist_to_cell_edge_mm, grid_disk, is_exportable,
};
use proxim_types::{EngineError, Position};

/// (lat_deg, lng_deg, [(res, expected index)]) from the H3 C library via Python h3 4.5.0.
type Vec1 = (f64, f64, &'static [(u8, u64)]);
const VECTORS: &[Vec1] = &[
    (
        37.3615593,
        -122.0553238,
        &[
            (0, 0x8029fffffffffff),
            (3, 0x832834fffffffff),
            (5, 0x85283473fffffff),
            (7, 0x87283472bffffff),
            (9, 0x89283470d93ffff),
            (12, 0x8c283470d921dff),
            (15, 0x8f283470d921c65),
        ],
    ),
    (
        48.864716,
        2.349014,
        &[
            (0, 0x8019fffffffffff),
            (3, 0x831fb4fffffffff),
            (5, 0x851fb467fffffff),
            (7, 0x871fb4662ffffff),
            (9, 0x891fb4662d7ffff),
            (12, 0x8c1fb4662d429ff),
            (15, 0x8f1fb4662d42804),
        ],
    ),
    (
        0.0,
        0.0,
        &[
            (0, 0x8075fffffffffff),
            (3, 0x83754efffffffff),
            (5, 0x85754e67fffffff),
            (7, 0x87754e64dffffff),
            (9, 0x89754e64993ffff),
            (12, 0x8c754e649929dff),
            (15, 0x8f754e64992d6d8),
        ],
    ),
    (
        -33.8688,
        151.2093,
        &[
            (0, 0x80bffffffffffff),
            (3, 0x83be0efffffffff),
            (5, 0x85be0e37fffffff),
            (7, 0x87be0e35cffffff),
            (9, 0x89be0e35cbbffff),
            (12, 0x8cbe0e35cbad1ff),
            (15, 0x8fbe0e35cbad0a8),
        ],
    ),
    (
        64.1466,
        -21.9426,
        &[
            (0, 0x8007fffffffffff),
            (3, 0x83075dfffffffff),
            (5, 0x85075dd7fffffff),
            (7, 0x87075dd4bffffff),
            (9, 0x89075dd4b8bffff),
            (12, 0x8c075dd4b890dff),
            (15, 0x8f075dd4b890c2e),
        ],
    ),
    (
        35.6762,
        139.6503,
        &[
            (0, 0x802ffffffffffff),
            (3, 0x832f5afffffffff),
            (5, 0x852f5a37fffffff),
            (7, 0x872f5a363ffffff),
            (9, 0x892f5a363bbffff),
            (12, 0x8c2f5a363ba01ff),
            (15, 0x8f2f5a363ba005a),
        ],
    ),
    (
        -0.0001,
        179.9999,
        &[
            (0, 0x807ffffffffffff),
            (3, 0x837eb5fffffffff),
            (5, 0x857eb573fffffff),
            (7, 0x877eb5722ffffff),
            (9, 0x897eb57221bffff),
            (12, 0x8c7eb57221a05ff),
            (15, 0x8f7eb57221a0454),
        ],
    ),
    (
        51.5074,
        -0.1278,
        &[
            (0, 0x8019fffffffffff),
            (3, 0x83194afffffffff),
            (5, 0x85194ad3fffffff),
            (7, 0x87195da49ffffff),
            (9, 0x89195da49b7ffff),
            (12, 0x8c195da49a2d9ff),
            (15, 0x8f195da49a2d8ca),
        ],
    ),
    (
        79.9,
        10.0,
        &[
            (0, 0x8001fffffffffff),
            (3, 0x83015afffffffff),
            (5, 0x85015aaffffffff),
            (7, 0x87015aac6ffffff),
            (9, 0x89015aac6b7ffff),
            (12, 0x8c015aac6b093ff),
            (15, 0x8f015aac6b09259),
        ],
    ),
    (
        -45.0,
        -170.0,
        &[
            (0, 0x80d5fffffffffff),
            (3, 0x83d442fffffffff),
            (5, 0x85d45527fffffff),
            (7, 0x87d442536ffffff),
            (9, 0x89d44253697ffff),
            (12, 0x8cd4425369625ff),
            (15, 0x8fd44253696256a),
        ],
    ),
];

const SF5: u64 = 0x85283473fffffff;
const PENTAGON_RES0: u64 = 0x8009fffffffffff;

#[test]
fn cell_of_matches_the_reference_library_at_every_resolution() {
    for (lat, lng, expect) in VECTORS {
        let p = pos(*lat, *lng);
        for (res, want) in *expect {
            assert_eq!(cell_of(p, *res), Ok(*want), "({lat}, {lng}) res {res}");
            assert_eq!(cell_resolution(*want), Ok(*res));
        }
    }
}

#[test]
fn cell_of_rejects_bad_arguments() {
    let p = pos(10.0, 10.0);
    assert_eq!(cell_of(p, 16), Err(EngineError::Parse));
    assert_eq!(cell_of(p, u8::MAX), Err(EngineError::Parse));
    for bad in [
        Position {
            lat_e7: 900_000_001,
            lon_e7: 0,
        },
        Position {
            lat_e7: -900_000_001,
            lon_e7: 0,
        },
        Position {
            lat_e7: 0,
            lon_e7: 1_800_000_001,
        },
        Position {
            lat_e7: i32::MAX,
            lon_e7: i32::MIN,
        },
    ] {
        assert_eq!(cell_of(bad, 5), Err(EngineError::Parse), "{bad:?}");
    }
    // The poles themselves are valid input for H3.
    assert!(
        cell_of(
            Position {
                lat_e7: 900_000_000,
                lon_e7: 0
            },
            5
        )
        .is_ok()
    );
    assert!(
        cell_of(
            Position {
                lat_e7: -900_000_000,
                lon_e7: 1_800_000_000
            },
            5
        )
        .is_ok()
    );
}

#[test]
fn export_guard_is_resolution_5() {
    assert_eq!(MAX_EXPORT_RES, 5);
    assert!(is_exportable(0x8029fffffffffff)); // res 0
    assert!(is_exportable(SF5));
    assert!(!is_exportable(0x87283472bffffff)); // res 7
    assert!(!is_exportable(0x8f283470d921c65)); // res 15
    assert!(!is_exportable(cell_of(pos(37.36, -122.05), 6).unwrap()));
    assert!(!is_exportable(0));
    assert!(!is_exportable(u64::MAX));
    assert_eq!(cell_resolution(0), Err(EngineError::Parse));
}

#[test]
fn grid_disk_counts_order_and_pentagon() {
    let sizes: Vec<usize> = (0..4).map(|k| grid_disk(SF5, k).unwrap().len()).collect();
    assert_eq!(sizes, vec![1, 7, 19, 37]);
    let pent: Vec<usize> = (0..4)
        .map(|k| grid_disk(PENTAGON_RES0, k).unwrap().len())
        .collect();
    assert_eq!(pent, vec![1, 6, 16, 31], "a pentagon has 5 neighbours");
    for k in 0..4 {
        let d = grid_disk(SF5, k).unwrap();
        assert!(
            d.windows(2).all(|w| w[0] < w[1]),
            "sorted ascending, no duplicates"
        );
        assert!(d.contains(&SF5));
        // Nested.
        let inner = grid_disk(SF5, k.saturating_sub(1)).unwrap();
        assert!(inner.iter().all(|c| d.contains(c)));
    }
    assert_eq!(grid_disk(SF5, 100).unwrap().len(), 30_301);
    assert_eq!(grid_disk(SF5, 101), Err(EngineError::Limit));
    assert_eq!(grid_disk(SF5, u32::MAX), Err(EngineError::Limit));
    assert_eq!(grid_disk(0, 1), Err(EngineError::Parse));
    assert_eq!(grid_disk(u64::MAX, 1), Err(EngineError::Parse));
}

#[test]
fn center_and_boundary_match_the_reference_library() {
    // Reference: center (37.34579337536848, -121.9763759725512) and first boundary vertex
    // (37.2713558667319, -121.91508032705622) from the C library, rounded to e7.
    let c = cell_center(SF5).unwrap();
    assert_eq!(
        c,
        Position {
            lat_e7: 373_457_934,
            lon_e7: -1_219_763_760
        }
    );
    let b = cell_boundary(SF5).unwrap();
    assert_eq!(b.len(), 6);
    assert_eq!(
        b[0],
        Position {
            lat_e7: 372_713_559,
            lon_e7: -1_219_150_803
        }
    );
    assert_eq!(
        b[5],
        Position {
            lat_e7: 372_631_980,
            lon_e7: -1_220_291_013
        }
    );
    assert_eq!(cell_of(c, 5), Ok(SF5), "the centre is inside its own cell");
    // A pentagon has 5 vertices (at least; edge-crossing cells can have more).
    assert!(cell_boundary(PENTAGON_RES0).unwrap().len() >= 5);
}

#[test]
fn distance_to_cell_edge() {
    let centre = cell_center(SF5).unwrap();
    let d = dist_to_cell_edge_mm(centre, SF5).unwrap();
    // Apothem of a res-5 cell: about 7.4 to 8.6 km.
    assert!((7_000_000..9_500_000).contains(&d), "{d}");
    // On a vertex and on an edge midpoint: ~0 (e7 rounding, 11 mm per unit).
    let v = cell_boundary(SF5).unwrap();
    assert!(dist_to_cell_edge_mm(v[0], SF5).unwrap() <= 15);
    let mid = Position {
        lat_e7: i32::try_from((i64::from(v[0].lat_e7) + i64::from(v[1].lat_e7)) / 2).unwrap(),
        lon_e7: i32::try_from((i64::from(v[0].lon_e7) + i64::from(v[1].lon_e7)) / 2).unwrap(),
    };
    assert!(dist_to_cell_edge_mm(mid, SF5).unwrap() <= 20);
    // 2 km from an edge, inside: measure along the line from centre to the edge midpoint.
    let frac = |f: i64| Position {
        lat_e7: i32::try_from(
            i64::from(centre.lat_e7) + (i64::from(mid.lat_e7) - i64::from(centre.lat_e7)) * f / 100,
        )
        .unwrap(),
        lon_e7: i32::try_from(
            i64::from(centre.lon_e7) + (i64::from(mid.lon_e7) - i64::from(centre.lon_e7)) * f / 100,
        )
        .unwrap(),
    };
    let near = dist_to_cell_edge_mm(frac(75), SF5).unwrap();
    let nearer = dist_to_cell_edge_mm(frac(90), SF5).unwrap();
    assert!(near > nearer && nearer > 0, "{near} {nearer}");
    // Distance from a point outside the cell is the distance to the boundary.
    let outside = dist_to_cell_edge_mm(frac(130), SF5).unwrap();
    assert!(outside > 0);
    // Typed errors: bad cell, polar point, cell beyond the 150 km frame (res 0).
    assert_eq!(dist_to_cell_edge_mm(centre, 0), Err(EngineError::Parse));
    assert_eq!(
        dist_to_cell_edge_mm(
            Position {
                lat_e7: 850_000_000,
                lon_e7: 0
            },
            SF5
        ),
        Err(EngineError::UnsupportedLatitude)
    );
    assert_eq!(
        dist_to_cell_edge_mm(centre, 0x8029fffffffffff),
        Err(EngineError::OutOfFrame)
    );
}

fn count_ring(center: Position, r_m: i64, res: u8) -> Vec<u64> {
    cover_circle(center, r_m * 1000, res).unwrap()
}

#[test]
fn cover_basic_shape_and_monotonicity() {
    let c = cell_center(SF5).unwrap();
    // A tiny circle around a cell centre at res 5 is just that cell (margin 100 m << 7 km).
    assert_eq!(count_ring(c, 0, 5), vec![SF5]);
    assert_eq!(count_ring(c, 500, 5), vec![SF5]);
    // Sorted, unique, contains the home cell, a subset of the surrounding disk.
    let v = count_ring(c, 12_000, 5);
    assert!(v.windows(2).all(|w| w[0] < w[1]));
    assert!(v.contains(&SF5));
    let disk = grid_disk(SF5, 4).unwrap();
    assert!(v.iter().all(|x| disk.contains(x)));
    // Radius monotone: a bigger circle covers a superset.
    let a = count_ring(c, 3_000, 7);
    let b = count_ring(c, 6_000, 7);
    assert!(a.iter().all(|x| b.contains(x)));
    assert!(a.len() < b.len());
    // Reasonable size: a 5 km circle at res 7 (cell area ~5.2 km2) needs roughly 15-60 cells.
    let five = count_ring(c, 5_000, 7);
    assert!((12..=70).contains(&five.len()), "{}", five.len());
    // Same inputs, same output.
    assert_eq!(five, count_ring(c, 5_000, 7));
}

#[test]
fn cover_margin_is_100_m_plus_a_sixteenth_of_the_radius() {
    // From the centre of a res-5 cell, the nearest cell edge is d_min away (~8 km). Cells across it
    // are excluded while r + margin < d_min and included once r + margin >= d_min, where
    // margin = 100 m + r / 16 (about 560 m at this radius). This pins the margin from both sides.
    let c = cell_center(SF5).unwrap();
    let d_min = dist_to_cell_edge_mm(c, SF5).unwrap();
    let only_home = cover_circle(c, d_min - 800_000, 5).unwrap();
    assert_eq!(
        only_home,
        vec![SF5],
        "0.8 km short of the edge: margin 0.56 km is not enough"
    );
    let with_neighbour = cover_circle(c, d_min - 400_000, 5).unwrap();
    assert!(
        with_neighbour.len() >= 2 && with_neighbour.contains(&SF5),
        "0.4 km short: margin reaches"
    );
    assert!(
        with_neighbour.len() < 7,
        "but not the whole ring: {}",
        with_neighbour.len()
    );
    // The constant 100 m part, isolated with r = 0 (so r / 16 = 0): a point 50 m inside the edge
    // pulls the neighbour in, a point 150 m inside does not.
    let v = cell_boundary(SF5).unwrap();
    let mid = Position {
        lat_e7: i32::try_from((i64::from(v[0].lat_e7) + i64::from(v[1].lat_e7)) / 2).unwrap(),
        lon_e7: i32::try_from((i64::from(v[0].lon_e7) + i64::from(v[1].lon_e7)) / 2).unwrap(),
    };
    let at = |ppm: i64| Position {
        lat_e7: i32::try_from(
            i64::from(c.lat_e7) + (i64::from(mid.lat_e7) - i64::from(c.lat_e7)) * ppm / 1_000_000,
        )
        .unwrap(),
        lon_e7: i32::try_from(
            i64::from(c.lon_e7) + (i64::from(mid.lon_e7) - i64::from(c.lon_e7)) * ppm / 1_000_000,
        )
        .unwrap(),
    };
    let first_within = |limit_mm: i64| {
        (900_000..1_000_000)
            .step_by(250)
            .map(at)
            .find(|p| dist_to_cell_edge_mm(*p, SF5).unwrap() <= limit_mm)
            .unwrap()
    };
    let p50 = first_within(50_000);
    let p150 = first_within(150_000);
    let d50 = dist_to_cell_edge_mm(p50, SF5).unwrap();
    let d150 = dist_to_cell_edge_mm(p150, SF5).unwrap();
    assert!(
        (30_000..=50_000).contains(&d50) && (130_000..=150_000).contains(&d150),
        "{d50} {d150}"
    );
    assert_eq!(cell_of(p50, 5), Ok(SF5), "still inside the home cell");
    assert!(
        cover_circle(p50, 0, 5).unwrap().len() >= 2,
        "within the 100 m constant margin"
    );
    assert_eq!(
        cover_circle(p150, 0, 5).unwrap(),
        vec![SF5],
        "beyond 100 m: home cell only"
    );
    // Exactly on the edge distance is certainly in, and all six neighbours join well beyond it.
    assert!(cover_circle(c, d_min, 5).unwrap().len() >= 2);
    assert_eq!(cover_circle(c, d_min * 3 / 2, 5).unwrap().len(), 7);
}

#[test]
fn cover_rejects_bad_arguments() {
    let c = pos(37.36, -122.05);
    assert_eq!(cover_circle(c, -1, 5), Err(EngineError::Parse));
    assert_eq!(cover_circle(c, 50_000_001, 5), Err(EngineError::Limit));
    assert!(cover_circle(c, 50_000_000, 4).is_ok());
    assert_eq!(cover_circle(c, 1000, 16), Err(EngineError::Parse));
    assert_eq!(
        cover_circle(
            Position {
                lat_e7: 850_000_000,
                lon_e7: 0
            },
            1000,
            5
        ),
        Err(EngineError::UnsupportedLatitude)
    );
    // Resolution too fine for the radius: bounded, not a billion-cell allocation.
    assert_eq!(cover_circle(c, 10_000_000, 15), Err(EngineError::Limit));
    assert_eq!(cover_circle(c, 50_000_000, 9), Err(EngineError::Limit));
    // Coarse resolutions never panic: cells larger than the frame are kept conservatively.
    let coarse = cover_circle(c, 1_000, 0).unwrap();
    assert!(coarse.contains(&0x8029fffffffffff));
    assert!(!cover_circle(c, 1_000, 2).unwrap().is_empty());
}

#[test]
fn cover_across_the_antimeridian_and_near_the_pole() {
    let c = pos(0.0, 179.9999);
    let east = cell_of(c, 5).unwrap();
    let west = cell_of(pos(0.0, -179.9999), 5).unwrap();
    let v = cover_circle(c, 5_000_000, 5).unwrap();
    assert!(
        v.contains(&east) && v.contains(&west),
        "both sides of the date line"
    );
    let hi = pos(79.9, 10.0);
    let v = cover_circle(hi, 10_000_000, 5).unwrap();
    assert!(v.contains(&cell_of(hi, 5).unwrap()));
    assert!(v.len() > 1);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(120))]

    /// Completeness: every point within the radius (spherical reference distance, which differs from
    /// the ellipsoidal frame by < 0.5%) lies in a cell of the cover.
    #[test]
    fn cover_contains_every_cell_of_points_inside_the_circle(
        lat in -75.0_f64..75.0, lon in -179.9_f64..179.9,
        r_m in 100_i64..9_000, res in 4_u8..=7,
        samples in prop::collection::vec((0.0_f64..360.0, 0.0_f64..1.0), 12),
    ) {
        let c = pos(lat, lon);
        let Ok(cover) = cover_circle(c, r_m * 1000, res) else {
            return Ok(()); // resolution too fine for this radius: a typed Limit, tested elsewhere
        };
        prop_assert!(cover.windows(2).all(|w| w[0] < w[1]));
        for (bearing, frac) in samples {
            // sqrt for a uniform spread over the disk; stay 1% inside to avoid rounding noise.
            let d = (r_m as f64) * 0.99 * libm::sqrt(frac);
            let q = destination(c, bearing, d);
            let cell = cell_of(q, res).unwrap();
            prop_assert!(cover.contains(&cell), "bearing {bearing} d {d}: cell {cell:x} missing");
        }
        // The centre's own cell is always there.
        prop_assert!(cover.contains(&cell_of(c, res).unwrap()));
    }

    /// Minimality (loose): every returned cell is within 2 km + 7% of the circle, in the true
    /// geometry, when the cell centre is measured on the sphere.
    #[test]
    fn cover_has_no_far_away_cells(
        lat in -70.0_f64..70.0, lon in -179.0_f64..179.0, r_m in 500_i64..8_000, res in 5_u8..=7,
    ) {
        let c = pos(lat, lon);
        let Ok(cover) = cover_circle(c, r_m * 1000, res) else { return Ok(()); };
        let f = frame_for(c).unwrap();
        for cell in cover {
            let ctr = cell_center(cell).unwrap();
            let q = f.project(ctr).unwrap();
            let dist = libm::hypot(f64::from(q.e_mm), f64::from(q.n_mm)) / 1000.0;
            // centre distance <= r + margin + circumradius (res 5 edge ~10 km, bound generously)
            let edge_m = [0.0, 0.0, 0.0, 0.0, 0.0, 10_000.0, 3_800.0, 1_450.0][usize::from(res)];
            prop_assert!(dist <= (r_m as f64) * 1.07 + 100.0 + 1.3 * edge_m, "dist {dist}");
        }
    }

    /// Total over extreme input: typed error or a sorted, non-empty result, never a panic.
    #[test]
    fn h3_functions_are_total(
        lat in any::<i32>(), lon in any::<i32>(), res in any::<u8>(), r in any::<i64>(),
        raw in any::<u64>(), k in any::<u32>(),
    ) {
        let p = Position { lat_e7: lat, lon_e7: lon };
        let _ = cell_of(p, res);
        let _ = cell_center(raw);
        let _ = cell_boundary(raw);
        let _ = cell_resolution(raw);
        let _ = dist_to_cell_edge_mm(p, raw);
        let _ = grid_disk(raw, k % 6); // keep the allowed-size work small
        if let Ok(v) = cover_circle(p, r.clamp(-10, 3_000_000), res % 8) {
            prop_assert!(!v.is_empty());
            prop_assert!(v.windows(2).all(|w| w[0] < w[1]));
        }
    }
}
