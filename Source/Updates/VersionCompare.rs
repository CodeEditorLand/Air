//! Semantic version comparison utility.
//!
//! `CompareVersions` parses two `"major.minor.patch"` strings and returns
//! `-1`, `0`, or `1` following the same contract as C's `strcmp` so callers
//! can use `match` on the result. Non-numeric segments are silently dropped.

/// Compare two semver strings.
/// Returns `1` if `v1 > v2`, `-1` if `v1 < v2`, `0` if equal.
pub fn CompareVersions(v1:&str, v2:&str) -> i32 {
	let v1_parts:Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();

	let v2_parts:Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();

	for (i, part) in v1_parts.iter().enumerate() {
		if i >= v2_parts.len() {
			return 1;
		}

		match part.cmp(&v2_parts[i]) {
			std::cmp::Ordering::Greater => return 1,

			std::cmp::Ordering::Less => return -1,

			std::cmp::Ordering::Equal => continue,
		}
	}

	if v1_parts.len() < v2_parts.len() { -1 } else { 0 }
}
