//! Combinatorial module optimizer.
//!
//! Faithful port of the scoring + search used by the StarResonanceAutoMod
//! project (`星痕共鸣模组筛选器`). It takes the player's parsed gear modules
//! (see [`crate::utils::modules`]) and finds the best combinations of 4 or 5
//! modules for a chosen set of attributes.
//!
//! ## Scoring (verified against AutoMod's logs)
//!
//! For a combination, each attribute's value is summed across the chosen
//! modules. That per-attribute sum is converted to a *level* by counting how
//! many of `ATTR_THRESHOLDS = [1, 4, 8, 12, 16, 20]` it meets (0..=6 — it caps
//! at 6 naturally because no threshold exceeds 20). The level maps to a power
//! value via the basic/special power tables. The score is the sum of those
//! per-attribute powers plus a bonus looked up from `TOTAL_ATTR_POWER_MAP`
//! keyed by the *uncapped* total of all attribute values.
//!
//! Note: like upstream, `TOTAL_ATTR_POWER_MAP` is sparse and missing keys
//! contribute `0` (`.get(total, 0)`) — this quirk is reproduced exactly.

use crate::utils::modules::{ATTRS, ModuleInfo, NUM_ATTRS, attr_slot};
use rayon::prelude::*;
use serde::Deserialize;
use specta::Type;
use std::collections::BinaryHeap;
use std::sync::{LazyLock, Mutex};

/// Persistent store of the local player's parsed gear modules.
///
/// Module data only arrives once (in `SyncContainerData` at login), but the
/// `Encounter` it used to live on is wiped on every server/scene change. This
/// dedicated state survives those resets so the optimizer keeps working until
/// the next login refreshes it.
pub type ModulesMutex = Mutex<Vec<ModuleInfo>>;

/// value → level breakpoints.
const ATTR_THRESHOLDS: [i32; 6] = [1, 4, 8, 12, 16, 20];
/// level (0..=6) → power, basic attributes.
const BASIC_ATTR_POWER: [i32; 7] = [0, 7, 14, 29, 44, 167, 254];
/// level (0..=6) → power, special "极" attributes.
const SPECIAL_ATTR_POWER: [i32; 7] = [0, 14, 29, 59, 89, 298, 448];

/// Sparse total-value → bonus pairs (verbatim from upstream `module_types.py`).
/// Keys not listed (9..=17, 107..=112, and >120) contribute 0.
const TOTAL_ATTR_POWER_PAIRS: &[(usize, i32)] = &[
    (0, 0),
    (1, 5),
    (2, 11),
    (3, 17),
    (4, 23),
    (5, 29),
    (6, 34),
    (7, 40),
    (8, 46),
    (18, 104),
    (19, 110),
    (20, 116),
    (21, 122),
    (22, 128),
    (23, 133),
    (24, 139),
    (25, 145),
    (26, 151),
    (27, 157),
    (28, 163),
    (29, 168),
    (30, 174),
    (31, 180),
    (32, 186),
    (33, 192),
    (34, 198),
    (35, 203),
    (36, 209),
    (37, 215),
    (38, 221),
    (39, 227),
    (40, 233),
    (41, 238),
    (42, 244),
    (43, 250),
    (44, 256),
    (45, 262),
    (46, 267),
    (47, 273),
    (48, 279),
    (49, 285),
    (50, 291),
    (51, 297),
    (52, 302),
    (53, 308),
    (54, 314),
    (55, 320),
    (56, 326),
    (57, 332),
    (58, 337),
    (59, 343),
    (60, 349),
    (61, 355),
    (62, 361),
    (63, 366),
    (64, 372),
    (65, 378),
    (66, 384),
    (67, 390),
    (68, 396),
    (69, 401),
    (70, 407),
    (71, 413),
    (72, 419),
    (73, 425),
    (74, 431),
    (75, 436),
    (76, 442),
    (77, 448),
    (78, 454),
    (79, 460),
    (80, 466),
    (81, 471),
    (82, 477),
    (83, 483),
    (84, 489),
    (85, 495),
    (86, 500),
    (87, 506),
    (88, 512),
    (89, 518),
    (90, 524),
    (91, 530),
    (92, 535),
    (93, 541),
    (94, 547),
    (95, 553),
    (96, 559),
    (97, 565),
    (98, 570),
    (99, 576),
    (100, 582),
    (101, 588),
    (102, 594),
    (103, 599),
    (104, 605),
    (105, 611),
    (106, 617),
    (113, 658),
    (114, 664),
    (115, 669),
    (116, 675),
    (117, 681),
    (118, 687),
    (119, 693),
    (120, 699),
];

/// Largest key in the total-value table.
const TOTAL_MAX: usize = 120;

/// `POWER_BY_VALUE[special][v]` = power for an attribute whose summed value is
/// `v` (already clamped to 0..=20). Index 0 = basic table, 1 = special table.
static POWER_BY_VALUE: LazyLock<[[i32; 21]; 2]> = LazyLock::new(|| {
    let mut table = [[0i32; 21]; 2];
    for (kind, powers) in [&BASIC_ATTR_POWER, &SPECIAL_ATTR_POWER]
        .into_iter()
        .enumerate()
    {
        for value in 0..=20i32 {
            let level = ATTR_THRESHOLDS.iter().filter(|&&t| value >= t).count();
            table[kind][value as usize] = powers[level];
        }
    }
    table
});

/// `TOTAL_LUT[t]` = total-value bonus, with sparse gaps filled with 0.
static TOTAL_LUT: LazyLock<[i32; TOTAL_MAX + 1]> = LazyLock::new(|| {
    let mut lut = [0i32; TOTAL_MAX + 1];
    for &(k, v) in TOTAL_ATTR_POWER_PAIRS {
        lut[k] = v;
    }
    lut
});

/// Whether attribute slot `i` is a special "极" attribute.
static IS_SPECIAL: LazyLock<[bool; NUM_ATTRS]> = LazyLock::new(|| {
    let mut s = [false; NUM_ATTRS];
    for (i, a) in ATTRS.iter().enumerate() {
        s[i] = a.special;
    }
    s
});

/// Score a combination from its summed per-attribute totals and the (uncapped)
/// total value. Returns the integer score (rendered as e.g. `1530.00` in the UI).
#[inline]
fn score_totals(totals: &[i32; NUM_ATTRS], total_value: i32) -> i64 {
    let power_by_value = &*POWER_BY_VALUE;
    let is_special = &*IS_SPECIAL;
    let mut threshold_power: i64 = 0;
    for i in 0..NUM_ATTRS {
        let v = totals[i];
        if v > 0 {
            let clamped = v.min(20) as usize;
            threshold_power += power_by_value[is_special[i] as usize][clamped] as i64;
        }
    }
    let total_power = if (0..=TOTAL_MAX as i32).contains(&total_value) {
        TOTAL_LUT[total_value as usize] as i64
    } else {
        0
    };
    threshold_power + total_power
}

// --- request / response types -------------------------------------------------

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeRequest {
    /// `"All"`, `"Attack"`, `"Guardian"`, or `"Support"`.
    pub category: String,
    /// Attribute ids to maximize (empty = maximize overall power).
    pub target_attrs: Vec<i32>,
    /// Attribute ids to avoid: any module carrying one of these is dropped.
    pub exclude_attrs: Vec<i32>,
    /// A module must carry at least this many of `target_attrs` to be used.
    pub match_count: u32,
    /// Combination size: 4 or 5.
    pub combo_size: u32,
    /// Number of ranked solutions to return.
    pub top_n: u32,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttrValue {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, serde::Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSolution {
    pub rank: u32,
    pub score: f64,
    pub total_value: i32,
    pub modules: Vec<ModuleInfo>,
    pub breakdown: Vec<AttrValue>,
}

// --- internal scored candidate ------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
struct Scored {
    score: i64,
    total: i32,
    idxs: Vec<usize>,
}

impl Ord for Scored {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .cmp(&other.score)
            .then(self.total.cmp(&other.total))
    }
}
impl PartialOrd for Scored {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Bounded min-heap that retains the `cap` highest-scoring items.
struct TopK {
    cap: usize,
    heap: BinaryHeap<std::cmp::Reverse<Scored>>,
}
impl TopK {
    fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            heap: BinaryHeap::new(),
        }
    }
    fn push(&mut self, item: Scored) {
        if self.heap.len() < self.cap {
            self.heap.push(std::cmp::Reverse(item));
        } else if let Some(std::cmp::Reverse(min)) = self.heap.peek() {
            if item > *min {
                self.heap.pop();
                self.heap.push(std::cmp::Reverse(item));
            }
        }
    }
    fn into_vec(self) -> Vec<Scored> {
        self.heap.into_iter().map(|r| r.0).collect()
    }
}

// --- public entry point -------------------------------------------------------

/// Run the optimizer. `modules` is the full parsed inventory; the result is the
/// top `top_n` solutions sorted by score descending.
pub fn optimize(modules: &[ModuleInfo], req: &OptimizeRequest) -> Vec<ModuleSolution> {
    let combo_size = req.combo_size.clamp(4, 5) as usize;
    let top_n = req.top_n.clamp(1, 100) as usize;

    // Resolve target / exclude attribute ids to canonical slots.
    let target_slots: Vec<usize> = req
        .target_attrs
        .iter()
        .filter_map(|&id| attr_slot(id))
        .collect();
    let exclude_ids: Vec<i32> = req.exclude_attrs.clone();

    // 1) Filter by category / exclude / match-count.
    let filtered: Vec<&ModuleInfo> = modules
        .iter()
        .filter(|m| match req.category.as_str() {
            "All" | "" => true,
            other => m.category.as_str() == other,
        })
        .filter(|m| !m.parts.iter().any(|p| exclude_ids.contains(&p.id)))
        .filter(|m| {
            if target_slots.is_empty() || req.match_count == 0 {
                return true;
            }
            let hits = m
                .parts
                .iter()
                .filter(|p| attr_slot(p.id).is_some_and(|s| target_slots.contains(&s)))
                .count();
            hits as u32 >= req.match_count
        })
        .collect();

    if filtered.len() < combo_size {
        return Vec::new();
    }

    // Precompute attribute vectors / totals / target-sums.
    let vecs: Vec<[i32; NUM_ATTRS]> = filtered.iter().map(|m| m.attr_vector()).collect();
    let totals_v: Vec<i32> = filtered.iter().map(|m| m.total_value()).collect();
    let target_sum: Vec<i32> = vecs
        .iter()
        .map(|v| target_slots.iter().map(|&s| v[s]).sum())
        .collect();

    // 2) Prefilter to a bounded candidate set.
    let candidates = prefilter(filtered.len(), &totals_v, &target_sum, &vecs, &target_slots);

    // Materialize candidate-local arrays.
    let cand_vecs: Vec<[i32; NUM_ATTRS]> = candidates.iter().map(|&i| vecs[i]).collect();
    let cand_tv: Vec<i32> = candidates.iter().map(|&i| totals_v[i]).collect();

    // 3) Enumerate.
    let scored = if combo_size <= 4 {
        enumerate_exact(&cand_vecs, &cand_tv, combo_size, top_n)
    } else {
        beam_search(&cand_vecs, &cand_tv, combo_size, top_n)
    };

    // 4) Build solutions (mapping candidate-local indices back to modules).
    scored
        .into_iter()
        .enumerate()
        .map(|(rank, s)| {
            let modules: Vec<ModuleInfo> = s
                .idxs
                .iter()
                .map(|&ci| filtered[candidates[ci]].clone())
                .collect();
            let mut totals = [0i32; NUM_ATTRS];
            for &ci in &s.idxs {
                for i in 0..NUM_ATTRS {
                    totals[i] += cand_vecs[ci][i];
                }
            }
            let mut breakdown: Vec<AttrValue> = (0..NUM_ATTRS)
                .filter(|&i| totals[i] > 0)
                .map(|i| AttrValue {
                    name: ATTRS[i].name.to_string(),
                    value: totals[i],
                })
                .collect();
            breakdown.sort_by(|a, b| b.value.cmp(&a.value));
            ModuleSolution {
                rank: rank as u32 + 1,
                score: s.score as f64,
                total_value: s.total,
                modules,
                breakdown,
            }
        })
        .collect()
}

/// Reduce the working set to a bounded list of candidate module indices.
fn prefilter(
    n: usize,
    totals_v: &[i32],
    target_sum: &[i32],
    vecs: &[[i32; NUM_ATTRS]],
    target_slots: &[usize],
) -> Vec<usize> {
    const ENUMERATION_NUM: usize = 500;
    let single_attr_num = if target_slots.len() <= 5 { 120 } else { 60 };

    // Top modules by (target-attribute sum, then overall sum).
    let mut by_total: Vec<usize> = (0..n).collect();
    by_total.sort_by(|&a, &b| {
        target_sum[b]
            .cmp(&target_sum[a])
            .then(totals_v[b].cmp(&totals_v[a]))
    });
    by_total.truncate(ENUMERATION_NUM);

    if target_slots.is_empty() {
        return by_total;
    }

    let mut keep: Vec<bool> = vec![false; n];
    for &i in &by_total {
        keep[i] = true;
    }
    // Plus the best modules for each individual target attribute.
    for &slot in target_slots {
        let mut by_attr: Vec<usize> = (0..n).filter(|&i| vecs[i][slot] > 0).collect();
        by_attr.sort_by(|&a, &b| vecs[b][slot].cmp(&vecs[a][slot]));
        for &i in by_attr.iter().take(single_attr_num) {
            keep[i] = true;
        }
    }
    (0..n).filter(|&i| keep[i]).collect()
}

/// Full parallel enumeration of all `size`-combinations (size ≤ 4).
fn enumerate_exact(
    vecs: &[[i32; NUM_ATTRS]],
    tv: &[i32],
    size: usize,
    top_n: usize,
) -> Vec<Scored> {
    let n = vecs.len();
    let merged: Vec<Scored> = (0..n)
        .into_par_iter()
        .map(|a| {
            let mut heap = TopK::new(top_n);
            let mut path = vec![a];
            recurse(vecs, tv, size, a, &mut path, vecs[a], tv[a], &mut heap, 1);
            heap.into_vec()
        })
        .reduce(Vec::new, |mut acc, mut v| {
            acc.append(&mut v);
            acc
        });
    finalize(merged, top_n)
}

/// Depth-first helper for [`enumerate_exact`].
#[allow(clippy::too_many_arguments)]
fn recurse(
    vecs: &[[i32; NUM_ATTRS]],
    tv: &[i32],
    size: usize,
    last: usize,
    path: &mut Vec<usize>,
    cur_totals: [i32; NUM_ATTRS],
    cur_tv: i32,
    heap: &mut TopK,
    depth: usize,
) {
    if depth == size {
        let score = score_totals(&cur_totals, cur_tv);
        heap.push(Scored {
            score,
            total: cur_tv,
            idxs: path.clone(),
        });
        return;
    }
    let n = vecs.len();
    for next in (last + 1)..n {
        let mut totals = cur_totals;
        for i in 0..NUM_ATTRS {
            totals[i] += vecs[next][i];
        }
        path.push(next);
        recurse(
            vecs,
            tv,
            size,
            next,
            path,
            totals,
            cur_tv + tv[next],
            heap,
            depth + 1,
        );
        path.pop();
    }
}

/// Beam search for larger combinations (size 5), where full enumeration is
/// intractable. Heuristic — results are near-optimal, matching upstream.
fn beam_search(vecs: &[[i32; NUM_ATTRS]], tv: &[i32], size: usize, top_n: usize) -> Vec<Scored> {
    const BEAM_WIDTH: usize = 5096;
    let n = vecs.len();

    #[derive(Clone)]
    struct State {
        totals: [i32; NUM_ATTRS],
        tv: i32,
        last: usize,
        idxs: Vec<usize>,
    }

    // Retain the `width` highest-scoring partial states.
    fn keep_top_states(states: &mut Vec<State>, width: usize) {
        if states.len() <= width {
            return;
        }
        states.sort_by(|a, b| {
            score_totals(&b.totals, b.tv)
                .cmp(&score_totals(&a.totals, a.tv))
                .then(b.tv.cmp(&a.tv))
        });
        states.truncate(width);
    }

    // Level 1: every single module.
    let mut beam: Vec<State> = (0..n)
        .map(|i| State {
            totals: vecs[i],
            tv: tv[i],
            last: i,
            idxs: vec![i],
        })
        .collect();
    keep_top_states(&mut beam, BEAM_WIDTH);

    for _ in 1..size {
        let mut next: Vec<State> = beam
            .par_iter()
            .flat_map_iter(|state| {
                ((state.last + 1)..n).map(move |k| {
                    let mut totals = state.totals;
                    for i in 0..NUM_ATTRS {
                        totals[i] += vecs[k][i];
                    }
                    let mut idxs = state.idxs.clone();
                    idxs.push(k);
                    State {
                        totals,
                        tv: state.tv + tv[k],
                        last: k,
                        idxs,
                    }
                })
            })
            .collect();
        keep_top_states(&mut next, BEAM_WIDTH);
        beam = next;
    }

    let scored: Vec<Scored> = beam
        .into_iter()
        .filter(|s| s.idxs.len() == size)
        .map(|s| Scored {
            score: score_totals(&s.totals, s.tv),
            total: s.tv,
            idxs: s.idxs,
        })
        .collect();
    finalize(scored, top_n)
}

/// Dedup by index-set, sort by score desc, and keep the top `top_n`.
fn finalize(mut scored: Vec<Scored>, top_n: usize) -> Vec<Scored> {
    scored.sort_by(|a, b| b.cmp(a));
    let mut seen = std::collections::HashSet::new();
    scored.retain(|s| {
        let mut key = s.idxs.clone();
        key.sort_unstable();
        seen.insert(key)
    });
    scored.truncate(top_n);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals_from(pairs: &[(i32, i32)]) -> ([i32; NUM_ATTRS], i32) {
        let mut t = [0i32; NUM_ATTRS];
        let mut tv = 0;
        for &(id, val) in pairs {
            t[attr_slot(id).unwrap()] += val;
            tv += val;
        }
        (t, tv)
    }

    #[test]
    fn score_matches_automod_log_rank1() {
        // Rank #1: Strength 21, Attack SPD 21, Agility 21, Elite 20, DMG Stack 3 → 1530.
        let (t, tv) = totals_from(&[
            (1110, 21), // Strength Boost
            (1408, 21), // Attack SPD
            (1111, 21), // Agility Boost
            (1114, 20), // Elite Strike
            (2104, 3),  // DMG Stack (special)
        ]);
        assert_eq!(tv, 86);
        assert_eq!(score_totals(&t, tv), 1530);
    }

    #[test]
    fn score_matches_automod_log_rank8() {
        // Rank #8: Strength 16, Attack SPD 21, Agility 21, DMG Stack 8, Elite 20 → 1488.
        let (t, tv) = totals_from(&[(1110, 16), (1408, 21), (1111, 21), (2104, 8), (1114, 20)]);
        assert_eq!(tv, 86);
        assert_eq!(score_totals(&t, tv), 1488);
    }

    #[test]
    fn missing_total_key_contributes_zero() {
        // total = 10 is a gap in the upstream table → total bonus is 0.
        let (t, tv) = totals_from(&[(1110, 10)]); // Strength 10 → level 3 → 29
        assert_eq!(tv, 10);
        assert_eq!(score_totals(&t, tv), 29);
    }
}
