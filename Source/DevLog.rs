//! # DevLog - Tag-filtered development logging
//!
//! Controlled by `Trace` environment variable.
//! The same tags work in Mountain (Rust), Air (Rust), Wind/Sky (TypeScript).
//!
//! ## Usage
//! ```bash
//! Trace=lifecycle,grpc ./Air          # only lifecycle + gRPC
//! Trace=all ./Air                     # everything
//! Trace=short ./Air                   # everything, compressed + deduped
//! Trace=indexing,http ./Air           # indexing + HTTP
//! ./Air                                      # nothing (silent daemon)
//! ```
//!
//! ## Short Mode
//!
//! `Trace=short` enables all tags but compresses output:
//! - Long app-data paths aliased to `$APP`
//! - Consecutive duplicate messages counted (`(x14)` suffix)
//! - Rust log targets compressed (`D::Binary::Main::Entry` → `Entry`)
//!
//! ## Tags (Air daemon tags)
//!
//! | Tag           | Scope                                               |
//! |---------------|-----------------------------------------------------|
//! | `vfs`         | File stat, read, write, readdir, mkdir, delete, copy|
//! | `ipc`         | IPC routing: invoke dispatch, channel calls          |
//! | `config`      | Configuration get/set, env paths, workbench config   |
//! | `lifecycle`   | Startup, shutdown, phases, window events             |
//! | `storage`     | Storage get/set/delete, items, optimize              |
//! | `extensions`  | Extension scanning, activation, management           |
//! | `update`      | Update service: check, download, apply               |
//! | `grpc`        | gRPC/Vine: server, client, connections               |
//! | `indexing`    | File system indexing, symbol extraction, watching    |
//! | `http`        | HTTP client requests, responses, retries             |
//! | `daemon`      | Daemon lifecycle, lock management, signals           |
//! | `security`    | Rate limiting, checksums, secure storage             |
//! | `metrics`     | Prometheus-style metrics collection                  |
//! | `air`         | Air tracing, telemetry, OTLP emission                |
//! | `resilience`  | Retry policies, circuit breakers, timeouts           |
//! | `bootstrap`   | Effect-TS bootstrap stages                           |

use std::sync::{Mutex, OnceLock};

static ENABLED_TAGS:OnceLock<Vec<String>> = OnceLock::new();
static SHORT_MODE:OnceLock<bool> = OnceLock::new();

// ── Path alias ──────────────────────────────────────────────────────────
// The app-data directory name is absurdly long. In short mode, alias it.
static APP_DATA_PREFIX:OnceLock<Option<String>> = OnceLock::new();

fn DetectAppDataPrefix() -> Option<String> {
	// Match the bundle identifier pattern used by Air/Mountain
	if let Ok(Home) = std::env::var("HOME") {
		let Base = format!("{}/Library/Application Support", Home);
		if let Ok(Entries) = std::fs::read_dir(&Base) {
			for Entry in Entries.flatten() {
				let Name = Entry.file_name();
				let Name = Name.to_string_lossy();
				if Name.starts_with("land.editor.") && Name.contains("mountain") {
					return Some(format!("{}/{}", Base, Name));
				}
			}
		}
	}
	None
}

/// Get the app-data path prefix for aliasing (cached).
pub fn AppDataPrefix() -> &'static Option<String> { APP_DATA_PREFIX.get_or_init(DetectAppDataPrefix) }

/// Replace the long app-data path with `$APP` in a string.
pub fn AliasPath(Input:&str) -> String {
	if let Some(Prefix) = AppDataPrefix() {
		Input.replace(Prefix.as_str(), "$APP")
	} else {
		Input.to_string()
	}
}

// ── Dedup buffer ────────────────────────────────────────────────────────

pub struct DedupState {
	pub LastKey:String,
	pub Count:u64,
}

pub static DEDUP:Mutex<DedupState> = Mutex::new(DedupState { LastKey:String::new(), Count:0 });

/// Flush the dedup buffer - prints the pending count if > 1.
pub fn FlushDedup() {
	if let Ok(mut State) = DEDUP.lock() {
		if State.Count > 1 {
			eprintln!("  (x{})", State.Count);
		}
		State.LastKey.clear();
		State.Count = 0;
	}
}

// ── Tag resolution ──────────────────────────────────────────────────────

fn EnabledTags() -> &'static Vec<String> {
	ENABLED_TAGS.get_or_init(|| {
		match std::env::var("Trace") {
			Ok(Val) => Val.split(',').map(|S| S.trim().to_lowercase()).collect(),
			Err(_) => vec![],
		}
	})
}

/// Whether `Trace=short` is active.
pub fn IsShort() -> bool { *SHORT_MODE.get_or_init(|| EnabledTags().iter().any(|T| T == "short")) }

/// Check if a tag is enabled.
pub fn IsEnabled(Tag:&str) -> bool {
	let Tags = EnabledTags();
	if Tags.is_empty() {
		return false;
	}
	let Lower = Tag.to_lowercase();
	Tags.iter().any(|T| T == "all" || T == "short" || T == Lower.as_str())
}

/// Log a tagged dev message. Only prints if the tag is enabled via
/// Trace.
///
/// In `short` mode: aliases long paths, deduplicates consecutive identical
/// lines.
#[macro_export]
macro_rules! dev_log {
	($Tag:expr, $($Arg:tt)*) => {
		if $crate::DevLog::IsEnabled($Tag) {
			let RawMessage = format!($($Arg)*);
			let TagUpper = $Tag.to_uppercase();
			if $crate::DevLog::IsShort() {
				let Aliased = $crate::DevLog::AliasPath(&RawMessage);
				let Key = format!("{}:{}", TagUpper, Aliased);
				let ShouldPrint = {
					if let Ok(mut State) = $crate::DevLog::DEDUP.lock() {
						if State.LastKey == Key {
							State.Count += 1;
							false
						} else {
							let PrevCount = State.Count;
							let HadPrev = !State.LastKey.is_empty();
							State.LastKey = Key;
							State.Count = 1;
							if HadPrev && PrevCount > 1 {
								eprintln!("  (x{})", PrevCount);
							}
							true
						}
					} else {
						true
					}
				};
				if ShouldPrint {
					eprintln!("[DEV:{}] {}", TagUpper, Aliased);
				}
			} else {
				eprintln!("[DEV:{}] {}", TagUpper, RawMessage);
			}
		}
	};
}

// ============================================================================
// OTLP Span Emission - sends spans directly to Jaeger/OTEL collector
// ============================================================================

use std::{
	sync::atomic::{AtomicBool, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

static OTLP_AVAILABLE:AtomicBool = AtomicBool::new(true);
static OTLP_TRACE_ID:OnceLock<String> = OnceLock::new();

fn GetTraceId() -> &'static str {
	OTLP_TRACE_ID.get_or_init(|| {
		use std::{
			collections::hash_map::DefaultHasher,
			hash::{Hash, Hasher},
		};
		let mut H = DefaultHasher::new();
		std::process::id().hash(&mut H);
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos()
			.hash(&mut H);
		format!("{:032x}", H.finish() as u128)
	})
}

pub fn NowNano() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }

/// Emit an OTLP span to the local collector (Jaeger at 127.0.0.1:4318).
/// Fire-and-forget on a background thread. Stops trying after first failure.
pub fn EmitOTLPSpan(Name:&str, StartNano:u64, EndNano:u64, Attributes:&[(&str, &str)]) {
	if !cfg!(debug_assertions) {
		return;
	}
	if !OTLP_AVAILABLE.load(Ordering::Relaxed) {
		return;
	}

	let SpanId = format!("{:016x}", rand_u64());
	let TraceId = GetTraceId().to_string();
	let SpanName = Name.to_string();

	let AttributesJson:Vec<String> = Attributes
		.iter()
		.map(|(K, V)| {
			format!(
				r#"{{"key":"{}","value":{{"stringValue":"{}"}}}}"#,
				K,
				V.replace('\\', "\\\\").replace('"', "\\\"")
			)
		})
		.collect();

	let IsError = SpanName.contains("error");

	let StatusCode = if IsError { 2 } else { 1 };
	let Payload = format!(
		concat!(
			r#"{{"resourceSpans":[{{"resource":{{"attributes":["#,
			r#"{{"key":"service.name","value":{{"stringValue":"land-editor-air"}}}},"#,
			r#"{{"key":"service.version","value":{{"stringValue":"0.0.1"}}}}"#,
			r#"]}},"scopeSpans":[{{"scope":{{"name":"air.daemon","version":"1.0.0"}},"#,
			r#""spans":[{{"traceId":"{}","spanId":"{}","name":"{}","kind":1,"#,
			r#""startTimeUnixNano":"{}","endTimeUnixNano":"{}","#,
			r#""attributes":[{}],"status":{{"code":{}}}}}]}}]}}]}}"#,
		),
		TraceId,
		SpanId,
		SpanName,
		StartNano,
		EndNano,
		AttributesJson.join(","),
		StatusCode,
	);

	// Fire-and-forget on a background thread
	std::thread::spawn(move || {
		use std::{
			io::{Read as IoRead, Write as IoWrite},
			net::TcpStream,
			time::Duration,
		};

		let Ok(mut Stream) = TcpStream::connect_timeout(&"127.0.0.1:4318".parse().unwrap(), Duration::from_millis(200))
		else {
			OTLP_AVAILABLE.store(false, Ordering::Relaxed);
			return;
		};
		let _ = Stream.set_write_timeout(Some(Duration::from_millis(200)));
		let _ = Stream.set_read_timeout(Some(Duration::from_millis(200)));

		let HttpReq = format!(
			"POST /v1/traces HTTP/1.1\r\nHost: 127.0.0.1:4318\r\nContent-Type: application/json\r\nContent-Length: \
			 {}\r\nConnection: close\r\n\r\n",
			Payload.len()
		);
		if Stream.write_all(HttpReq.as_bytes()).is_err() {
			return;
		}
		if Stream.write_all(Payload.as_bytes()).is_err() {
			return;
		}
		let mut Buf = [0u8; 32];
		let _ = Stream.read(&mut Buf);
		if !(Buf.starts_with(b"HTTP/1.1 2") || Buf.starts_with(b"HTTP/1.0 2")) {
			OTLP_AVAILABLE.store(false, Ordering::Relaxed);
		}
	});
}

fn rand_u64() -> u64 {
	use std::{
		collections::hash_map::DefaultHasher,
		hash::{Hash, Hasher},
	};
	let mut H = DefaultHasher::new();
	std::thread::current().id().hash(&mut H);
	NowNano().hash(&mut H);
	H.finish()
}

/// Convenience macro: emit an OTLP span for an Air daemon handler.
/// Usage: `otel_span!("air:indexFile", StartNano, &[("path", &SomePath)]);`
#[macro_export]
macro_rules! otel_span {
	($Name:expr, $Start:expr, $Attrs:expr) => {
		$crate::DevLog::EmitOTLPSpan($Name, $Start, $crate::DevLog::NowNano(), $Attrs)
	};
	($Name:expr, $Start:expr) => {
		$crate::DevLog::EmitOTLPSpan($Name, $Start, $crate::DevLog::NowNano(), &[])
	};
}
