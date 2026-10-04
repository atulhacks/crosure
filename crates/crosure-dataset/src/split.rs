//! Train/test splits that do not leak.
//!
//! Two investigations of the same binary, or of binaries that share real
//! code (same function fingerprint), must land on the same side: otherwise a
//! model is tested on functions it was trained on. Trajectories are grouped
//! by binary and by significant shared fingerprints, then each group is
//! assigned to a split by a hash of its id, so re-exporting gives the same
//! split. Fingerprints common to many binaries (statically linked library
//! code) do not join groups, after Ghidra FunctionID's "auto-fail common".

use std::collections::{BTreeMap, BTreeSet};

use crosure_engine::FunctionHash;
use sha2::{Digest, Sha256};

use crate::record::Trajectory;

/// Instructions a shared function needs to tie two trajectories together.
pub const MIN_SHARED_INSNS: usize = 12;

fn sha(s: &str) -> [u8; 32] {
    Sha256::digest(s.as_bytes()).into()
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Structural hashes of the significant functions a trajectory touched.
fn functions(t: &Trajectory) -> BTreeSet<String> {
    t.steps
        .iter()
        .filter_map(|s| s.func_fp.as_deref().and_then(FunctionHash::parse))
        .filter(|h| h.insns >= MIN_SHARED_INSNS)
        .map(|h| h.structural)
        .collect()
}

/// Sets `group` and `split` (`train` / `test`) on every trajectory. About
/// `test_percent`% of groups go to `test`.
///
/// ```
/// use crosure_dataset::{assign_splits, Trajectory};
/// let t = |id: &str, sha: &str| Trajectory {
///     format: "crosure.trajectory.v1".into(), session_id: id.into(), binary: "b".into(),
///     binary_sha256: sha.into(), head_hash: "h".into(), verified: true, steps: vec![],
///     group: String::new(), split: String::new(),
/// };
/// let mut ts = vec![t("a", "sha:1"), t("b", "sha:1"), t("c", "sha:2")];
/// assign_splits(&mut ts, 50);
/// assert_eq!(ts[0].group, ts[1].group, "same binary, same group");
/// assert_eq!(ts[0].split, ts[1].split);
/// assert_ne!(ts[0].group, ts[2].group);
/// ```
pub fn assign_splits(trajectories: &mut [Trajectory], test_percent: u8) {
    let n = trajectories.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let fns: Vec<BTreeSet<String>> = trajectories.iter().map(functions).collect();

    // A fingerprint seen in many distinct binaries is library code.
    let mut binaries_with: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (t, f) in trajectories.iter().zip(&fns) {
        for h in f {
            binaries_with.entry(h).or_default().insert(&t.binary_sha256);
        }
    }
    let distinct: BTreeSet<&str> = trajectories
        .iter()
        .map(|t| t.binary_sha256.as_str())
        .collect();
    let common_at = 3.max(distinct.len().div_ceil(10));

    let mut first_with: BTreeMap<String, usize> = BTreeMap::new();
    for (i, t) in trajectories.iter().enumerate() {
        let keys = std::iter::once(format!("bin:{}", t.binary_sha256)).chain(
            fns[i]
                .iter()
                .filter(|h| binaries_with.get(h.as_str()).map_or(0, BTreeSet::len) < common_at)
                .map(|h| format!("fn:{h}")),
        );
        for key in keys {
            match first_with.get(&key) {
                Some(&j) => {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                    parent[a] = b;
                }
                None => {
                    first_with.insert(key, i);
                }
            }
        }
    }

    // Group id: from its smallest binary hash, stable across exports.
    let mut root_id: BTreeMap<usize, String> = BTreeMap::new();
    for (i, t) in trajectories.iter().enumerate() {
        let r = find(&mut parent, i);
        let sha = t.binary_sha256.clone();
        root_id
            .entry(r)
            .and_modify(|m| {
                if sha < *m {
                    *m = sha.clone();
                }
            })
            .or_insert(sha);
    }
    for (i, t) in trajectories.iter_mut().enumerate() {
        let r = find(&mut parent, i);
        let digest = sha(root_id.get(&r).map_or("", String::as_str));
        let group: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
        let bucket = u16::from_be_bytes([digest[6], digest[7]]) % 100;
        t.split = if bucket < u16::from(test_percent) {
            "test"
        } else {
            "train"
        }
        .into();
        t.group = format!("g_{group}");
    }
}
