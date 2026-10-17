//! Wording rotation, policy H (`select_variant`, Addendum 02 5.1) and the pending-decision window
//! (5.3). Rust port of the normative reference `tools/rotation-ref/refkernel.py`; it must reproduce
//! `core/testdata/golden/rotation/golden_v2.json` and `golden_v2_newcount.json` exactly.
//!
//! Pure and integer-only: no clock, randomness or install salt (I26). The day number is the
//! device-local calendar date as days since 1970-01-05 (a Monday). Not yet called by `decide`:
//! step 7b is wired in M3.

use crate::sha256::sha256;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use proxim_types::EngineError;

pub const ONE: u32 = 256;
pub const SAT: u32 = 65_535;
pub const DISMISS: u32 = 512;
pub const PURGE_BELOW: u32 = 13;
pub const MAX_AGE: i64 = 90;
pub const SLACK: u32 = 256;
pub const K_MAX: usize = 3;
pub const RECENT_MAX: usize = 4;
pub const W_EMIT_MS: i64 = 60_000;
pub const LATE_MS: i64 = 600_000;
pub const LRU_DEFAULT: usize = 2000;
/// Shortest on-screen time that counts as a viewable showing (billing).
pub const VIEWABLE_MS: i64 = 1000;
pub const ROT_DOMAIN: &[u8] = b"PROXIM-ROT-V2\x00";

/// `DECAY[d] = round(65536 * 2^(-d/14))`, a 14-day half-life, as committed in golden_v2.json.
pub const DECAY: [u32; 91] = [
    65536, 62370, 59358, 56490, 53761, 51165, 48693, 46341, 44102, 41972, 39945, 38015, 36179,
    34431, 32768, 31185, 29679, 28245, 26881, 25582, 24347, 23170, 22051, 20986, 19972, 19008,
    18089, 17216, 16384, 15593, 14839, 14123, 13440, 12791, 12173, 11585, 11026, 10493, 9986, 9504,
    9045, 8608, 8192, 7796, 7420, 7061, 6720, 6396, 6087, 5793, 5513, 5247, 4993, 4752, 4522, 4304,
    4096, 3898, 3710, 3531, 3360, 3198, 3043, 2896, 2756, 2623, 2497, 2376, 2261, 2152, 2048, 1949,
    1855, 1765, 1680, 1599, 1522, 1448, 1378, 1312, 1248, 1188, 1131, 1076, 1024, 975, 927, 883,
    840, 799, 761,
];

fn u32_be(d: &[u8; 32]) -> u32 {
    u32::from_be_bytes([d[0], d[1], d[2], d[3]])
}

/// The public, per-family rotation inputs derived from the signed digests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    /// `vid_i`: first 4 bytes of creative digest i (ledger key only, never an authenticator).
    pub vids: Vec<u32>,
    /// `pos_i`: rank of `(key_i, i)`, `key_i = u32(SHA-256(ROT || "ord" || signpost_id || d_i))`.
    pub pos: Vec<u32>,
    /// `phi = u32(SHA-256(ROT || "phase" || signpost_id))`.
    pub phi: u32,
}

/// Derive vids, slot positions and phase.
///
/// # Errors
/// [`EngineError::Parse`] for an empty family, more than 8 wordings or two equal vids (the
/// compiler rejects them; the device fails closed).
pub fn derive(signpost_id: &[u8], digests: &[[u8; 32]]) -> Result<Family, EngineError> {
    if digests.is_empty() || digests.len() > 8 {
        return Err(EngineError::Parse);
    }
    let vids: Vec<u32> = digests.iter().map(u32_be).collect();
    let mut sorted = vids.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w.first() == w.get(1)) {
        return Err(EngineError::Parse);
    }
    let mut keyed: Vec<(u32, usize)> = digests
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut m = Vec::with_capacity(ROT_DOMAIN.len() + 3 + signpost_id.len() + 32);
            m.extend_from_slice(ROT_DOMAIN);
            m.extend_from_slice(b"ord");
            m.extend_from_slice(signpost_id);
            m.extend_from_slice(d);
            (u32_be(&sha256(&m)), i)
        })
        .collect();
    keyed.sort_unstable();
    let mut pos = alloc::vec![0u32; digests.len()];
    for (rank, (_, i)) in (0u32..).zip(keyed) {
        if let Some(p) = pos.get_mut(i) {
            *p = rank;
        }
    }
    let mut m = Vec::with_capacity(ROT_DOMAIN.len() + 5 + signpost_id.len());
    m.extend_from_slice(ROT_DOMAIN);
    m.extend_from_slice(b"phase");
    m.extend_from_slice(signpost_id);
    Ok(Family {
        vids,
        pos,
        phi: u32_be(&sha256(&m)),
    })
}

/// Index of `day` among the scheduled weekdays: `mask` bit 0 = Monday, 0 reads as every day.
/// The compiler writes 7 bits; like the reference kernel, a set bit 7 is counted, not masked off.
#[must_use]
pub fn sched_index(day: i64, mask: u8) -> i64 {
    let mask = if mask == 0 { 0x7f } else { mask };
    let dow = u32::try_from(day.rem_euclid(7)).unwrap_or(0);
    let before = u32::from(mask) & ((1u32 << dow) - 1);
    day.div_euclid(7)
        .saturating_mul(i64::from(mask.count_ones()))
        .saturating_add(i64::from(before.count_ones()))
}

/// The per-family exposure ledger on the device (memory and encrypted local store, no backup).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ledger {
    /// vid -> (count, last_day).
    pub rows: BTreeMap<u32, (u32, i64)>,
    /// Most recent vids, newest last, at most [`RECENT_MAX`].
    pub recent: Vec<u32>,
    pub last_day: Option<i64>,
    /// Row cap; the oldest row (then the smallest vid) is evicted, never the one just written.
    pub lru: usize,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            rows: BTreeMap::new(),
            recent: Vec::new(),
            last_day: None,
            lru: LRU_DEFAULT,
        }
    }
}

impl Ledger {
    /// Decayed count at `day` and whether a live row exists (evaluated at read, so ports agree).
    #[must_use]
    pub fn cn(&self, vid: u32, day: i64) -> (u32, bool) {
        let Some(&(count, last)) = self.rows.get(&vid) else {
            return (0, false);
        };
        let dt = day.saturating_sub(last).max(0);
        let Some(&f) = usize::try_from(dt).ok().and_then(|i| DECAY.get(i)) else {
            return (0, false);
        };
        let v = u32::try_from((u64::from(count) * u64::from(f)) >> 16).unwrap_or(SAT);
        if v >= PURGE_BELOW {
            (v, true)
        } else {
            (0, false)
        }
    }

    fn eff_recent(&self, day: i64) -> &[u32] {
        match self.last_day {
            Some(ld) if day.saturating_sub(ld) <= MAX_AGE => &self.recent,
            _ => &[],
        }
    }
}

/// A selection: the wording index and the count its commit will write (fixed at decision time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub idx: usize,
    pub new_count: u32,
}

/// Policy H. `avail_mask`: shipped, locale-matching and not variant-revoked wordings. `None` if no
/// wording is available.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn select(
    led: &Ledger,
    day: i64,
    fam: &Family,
    avail_mask: u8,
    dow_mask: u8,
    lane: u32,
    lanes: u32,
) -> Option<Selection> {
    let n = fam.vids.len();
    let a: Vec<usize> = (0..n).filter(|&i| avail_mask >> i & 1 == 1).collect();
    if a.is_empty() {
        return None;
    }
    let vid = |i: usize| fam.vids.get(i).copied().unwrap_or(0);
    let k = K_MAX.min(a.len() - 1);
    let recent = led.eff_recent(day);
    let banned = recent.get(recent.len().saturating_sub(k)..).unwrap_or(&[]);
    let banned = if k == 0 { &[][..] } else { banned };
    let cand: Vec<usize> = a
        .iter()
        .copied()
        .filter(|&i| !banned.contains(&vid(i)))
        .collect();
    let cs = |i: usize| led.cn(vid(i), day);
    let m = cand.iter().map(|&i| cs(i).0).min()?;
    let n_i = i128::try_from(n).ok()?;
    let lane_off = i128::from(lane) * n_i / i128::from(lanes.max(1));
    let t =
        (i128::from(sched_index(day, dow_mask)) + i128::from(fam.phi) + lane_off).rem_euclid(n_i);
    let best = cand
        .iter()
        .copied()
        .filter(|&i| cs(i).0 <= m.saturating_add(SLACK))
        .min_by_key(|&i| {
            let p = i128::from(fam.pos.get(i).copied().unwrap_or(0));
            (u8::from(cs(i).1), (p - t).rem_euclid(n_i), i)
        })?;
    let top = a
        .iter()
        .filter(|&&j| j != best)
        .map(|&j| cs(j).0)
        .max()
        .unwrap_or(0);
    let new_count = cs(best)
        .0
        .saturating_add(ONE)
        .max(top.saturating_add(1))
        .min(SAT);
    Some(Selection {
        idx: best,
        new_count,
    })
}

/// Write a committed showing: the row, the recent list and the family day; evict past `lru`.
pub fn commit(led: &mut Ledger, vid: u32, day: i64, new_count: u32) {
    led.rows.insert(vid, (new_count, day));
    led.recent.push(vid);
    let excess = led.recent.len().saturating_sub(RECENT_MAX);
    led.recent.drain(..excess);
    led.last_day = Some(day);
    if led.rows.len() > led.lru {
        let victim = led
            .rows
            .iter()
            .filter(|&(&k, _)| k != vid)
            .min_by_key(|&(&k, &(_, d))| (d, k))
            .map(|(&k, _)| k);
        if let Some(v) = victim {
            led.rows.remove(&v);
        }
    }
}

/// A dismissal rests the wording: `count = min(SAT, cn + DISMISS)`.
pub fn dismiss(led: &mut Ledger, vid: u32, day: i64) {
    let c = led.cn(vid, day).0;
    led.rows
        .insert(vid, (c.saturating_add(DISMISS).min(SAT), day));
}

/// How a pending decision ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Done {
    Open,
    /// Committed when the emission window expired (the host never called `markShown`).
    Emitted,
    /// Committed or billed by `markShown`.
    Marked,
}

/// The SDK's pending decision for one family (memory only, 5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    pub sel: Selection,
    pub t_ms: i64,
    /// The day the decision was made; every commit of it is written with this day.
    pub day: i64,
    pub done: Done,
}

/// What `markShown` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Ignored,
    Duplicate,
    Billed,
    CommittedNotViewable,
    BilledLate,
    TooLate,
}

impl Mark {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::Duplicate => "duplicate",
            Self::Billed => "billed",
            Self::CommittedNotViewable => "committed_not_viewable",
            Self::BilledLate => "billed_late",
            Self::TooLate => "too_late",
        }
    }
}

/// Write a pending decision's selection to the ledger under its decision day.
fn commit_pending(led: &mut Ledger, vids: &[u32], cur: &Pending) {
    if let Some(&vid) = vids.get(cur.sel.idx) {
        commit(led, vid, cur.day, cur.sel.new_count);
    }
}

/// Window expiry (5.3 rule 3), applied on every call (R-2026-10-17): an open decision whose
/// window has passed is committed as an emission. Returns whether it committed.
pub fn tick(p: &mut Option<Pending>, now_ms: i64, led: &mut Ledger, vids: &[u32]) -> bool {
    match p {
        Some(cur) if cur.done == Done::Open && now_ms.saturating_sub(cur.t_ms) >= W_EMIT_MS => {
            cur.done = Done::Emitted;
            commit_pending(led, vids, cur);
            true
        }
        _ => false,
    }
}

/// Ask for a wording on `day` (rule 1): inside the window an open decision is returned again.
/// Otherwise an expired open decision is first committed, then `choose` decides on the updated
/// ledger and its selection is held until commit. Returns the index and whether the expiry
/// committed.
pub fn emit(
    p: &mut Option<Pending>,
    now_ms: i64,
    day: i64,
    led: &mut Ledger,
    vids: &[u32],
    choose: impl FnOnce(&Ledger) -> Option<Selection>,
) -> (Option<usize>, bool) {
    if let Some(cur) = p
        && cur.done == Done::Open
        && now_ms.saturating_sub(cur.t_ms) < W_EMIT_MS
    {
        return (Some(cur.sel.idx), false);
    }
    let expired = tick(p, now_ms, led, vids);
    let Some(sel) = choose(led) else {
        return (None, expired);
    };
    *p = Some(Pending {
        sel,
        t_ms: now_ms,
        day,
        done: Done::Open,
    });
    (Some(sel.idx), expired)
}

/// The host reports the wording was on screen for `visible_ms`. Expiry is applied first, so a
/// report after the window can only bill (rule 4: within [`LATE_MS`], viewable, once) and never
/// commits a second time. Returns the outcome and whether the expiry committed.
pub fn mark_shown(
    p: &mut Option<Pending>,
    now_ms: i64,
    visible_ms: i64,
    led: &mut Ledger,
    vids: &[u32],
) -> (Mark, bool) {
    let expired = tick(p, now_ms, led, vids);
    let Some(cur) = p.as_mut() else {
        return (Mark::Ignored, expired);
    };
    let mark = match cur.done {
        Done::Marked => Mark::Duplicate,
        Done::Open => {
            cur.done = Done::Marked;
            commit_pending(led, vids, cur);
            if visible_ms >= VIEWABLE_MS {
                Mark::Billed
            } else {
                Mark::CommittedNotViewable
            }
        }
        Done::Emitted => {
            if now_ms.saturating_sub(cur.t_ms) <= LATE_MS && visible_ms >= VIEWABLE_MS {
                cur.done = Done::Marked;
                Mark::BilledLate
            } else {
                Mark::TooLate
            }
        }
    };
    (mark, expired)
}

/// Rotation day: days since 1970-01-05 (a Monday) from the engine's local `day_idx` (days since
/// 1970-01-01, a Thursday).
#[must_use]
pub const fn rot_day(day_idx: i64) -> i64 {
    day_idx.saturating_sub(4)
}

/// Parse a signpost's verified wordings into the rotation inputs.
///
/// # Errors
/// [`EngineError::Parse`] for a malformed digest, more than 8 or no wordings, a vid collision or
/// an availability mask without wording 0 or with bits beyond the family.
pub fn family_of(id: &str, w: &proxim_types::Wordings) -> Result<Family, EngineError> {
    let mut digests: Vec<[u8; 32]> = Vec::with_capacity(w.digests.len());
    for h in &w.digests {
        digests.push(parse_digest(h).ok_or(EngineError::Parse)?);
    }
    let n = digests.len();
    if w.avail_mask & 1 == 0 || (n < 8 && u16::from(w.avail_mask) >> n != 0) {
        return Err(EngineError::Parse);
    }
    derive(id.as_bytes(), &digests)
}

fn parse_digest(h: &str) -> Option<[u8; 32]> {
    let b = h.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (o, pair) in out.iter_mut().zip(b.chunks_exact(2)) {
        let [hi, lo] = pair else { return None };
        *o = (nib(*hi)? << 4) | nib(*lo)?;
    }
    Some(out)
}

/// All families' ledgers as held in `State.rotation`, with one global row limit
/// ([`LRU_DEFAULT`], Addendum 02 5.2): eviction removes the row with the smallest `last_day`,
/// then the smallest vid, then the smallest id, and never the row just written.
pub(crate) struct Book {
    rows: BTreeMap<(alloc::string::String, u32), (u32, i64)>,
    families: BTreeMap<alloc::string::String, (Vec<u32>, i64)>,
}

impl Book {
    /// Load, dropping what reads as absent at `day` anyway (rows and family state older than
    /// 90 days): deletion is lazy and unobservable (5.2).
    pub fn load(r: &proxim_types::Rotation, day: i64) -> Self {
        let fresh = |last: i64| day.saturating_sub(last) <= MAX_AGE;
        Self {
            rows: r
                .rows
                .iter()
                .filter(|x| fresh(x.last_day))
                .map(|x| ((x.id.as_str().into(), x.vid), (x.count, x.last_day)))
                .collect(),
            families: r
                .families
                .iter()
                .filter(|f| fresh(f.last_day))
                .map(|f| (f.id.as_str().into(), (f.recent.clone(), f.last_day)))
                .collect(),
        }
    }

    /// The ledger of one family, as the kernel sees it.
    pub fn view(&self, id: &str) -> Ledger {
        let rows = self
            .rows
            .range((id.into(), 0)..=(id.into(), u32::MAX))
            .map(|((_, vid), v)| (*vid, *v))
            .collect();
        let (recent, last_day) = self
            .families
            .get(id)
            .map_or((Vec::new(), None), |(r, d)| (r.clone(), Some(*d)));
        Ledger {
            rows,
            recent,
            last_day,
            // The global limit is applied by `store`.
            lru: usize::MAX,
        }
    }

    /// Write a family's ledger back; `written` is the vid the kernel just wrote, if any.
    pub fn store(&mut self, id: &proxim_types::Id, led: &Ledger, written: Option<u32>) {
        let key: alloc::string::String = id.as_str().into();
        self.rows.retain(|(fid, _), _| fid.as_str() != id.as_str());
        for (vid, v) in &led.rows {
            self.rows.insert((key.clone(), *vid), *v);
        }
        match led.last_day {
            Some(d) => {
                self.families.insert(key.clone(), (led.recent.clone(), d));
            }
            None => {
                self.families.remove(&key);
            }
        }
        while self.rows.len() > LRU_DEFAULT {
            let victim = self
                .rows
                .iter()
                .filter(|((fid, vid), _)| !(fid == &key && Some(*vid) == written))
                .min_by_key(|((fid, vid), (_, d))| (*d, *vid, fid.clone()))
                .map(|(k, _)| k.clone());
            match victim {
                Some(k) => {
                    self.rows.remove(&k);
                }
                None => break,
            }
        }
    }

    pub fn into_rotation(self) -> Result<proxim_types::Rotation, EngineError> {
        let mut rows = Vec::with_capacity(self.rows.len());
        for ((id, vid), (count, last_day)) in self.rows {
            rows.push(proxim_types::RotRow {
                id: proxim_types::Id::new(&id)?,
                vid,
                count,
                last_day,
            });
        }
        let mut families = Vec::with_capacity(self.families.len());
        for (id, (recent, last_day)) in self.families {
            families.push(proxim_types::RotFamily {
                id: proxim_types::Id::new(&id)?,
                recent,
                last_day,
            });
        }
        Ok(proxim_types::Rotation { rows, families })
    }
}

/// Structure of `State.rotation`: sorted and unique, within bounds.
pub(crate) fn validate(r: &proxim_types::Rotation) -> Result<(), EngineError> {
    if r.rows.len() > LRU_DEFAULT || r.families.len() > proxim_types::defaults::MAX_SIGNPOSTS {
        return Err(EngineError::Limit);
    }
    let rows_ok = r
        .rows
        .windows(2)
        .all(|w| matches!(w, [a, b] if (&a.id, a.vid) < (&b.id, b.vid)));
    let fams_ok = r
        .families
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.id < b.id))
        && r.families.iter().all(|f| f.recent.len() <= RECENT_MAX);
    if rows_ok && fams_ok {
        Ok(())
    } else {
        Err(EngineError::Parse)
    }
}
