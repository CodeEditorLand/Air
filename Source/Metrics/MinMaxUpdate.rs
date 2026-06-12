use std::sync::atomic::{AtomicU64, Ordering};

/// Helper function to update min/max values atomically
pub(crate) fn MinMaxUpdate(MinMetric:&AtomicU64, MaxMetric:&AtomicU64, Value:u64) {
	let mut CurrentMin = MinMetric.load(Ordering::Relaxed);

	let mut CurrentMax = MaxMetric.load(Ordering::Relaxed);

	loop {
		if Value < CurrentMin {
			match MinMetric.compare_exchange_weak(CurrentMin, Value, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => break,

				Err(NewMin) => CurrentMin = NewMin,
			}
		} else if Value > CurrentMax {
			match MaxMetric.compare_exchange_weak(CurrentMax, Value, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => break,

				Err(NewMax) => CurrentMax = NewMax,
			}
		} else {
			break;
		}
	}
}
