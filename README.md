<table>
	<tr>
		<td align="left" valign="middle">
			<h3 align="left">&#x2001;🪁</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">&#x2001;+&#x2001;</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">
					<picture>
						<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Land.svg">
						<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Land.svg">
						<img width="28" alt="Land Logo" src="https://PlayForm.Cloud/Image/GitHub/Land.svg">
					</picture>
				</a>
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">&#x2001;🏞️</a>
			</h3>
		</td>
	</tr>
</table>

---

# 🪁

The Native Background Daemon for `Land`🏞️.

> **`VS Code` cold-starts slowly because everything initializes fresh each
> launch. Updates require a full restart that kills open terminals and
> in-progress work. There is no mechanism to pre-stage work between sessions.**

_"The next version is already downloaded and verified before you decide to
update. The main window never blocks waiting for a download."_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](https://github.com/CodeEditorLand/Air/tree/Current/LICENSE)
[<img src="https://editor.land/Image/Rust.svg" width="14" alt="Rust" />](https://www.rust-lang.org/)&#x2001;[![Crates.io](https://img.shields.io/crates/v/Air.svg)](https://crates.io/crates/Air)
[![Rust Version](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

📖 **[Rust API Documentation](https://Rust.Documentation.Editor.Land/Air/)**

Welcome to **Air**🪁, the lightweight, persistent daemon that powers the
background capabilities of the **Land**🏞️ Code Editor. While `Mountain` handles
the core application logic and UI, **Air** operates as a specialized sidecar
process dedicated to heavy lifting, network operations, and system maintenance.
It ensures that the main editor remains responsive by offloading
resource-intensive tasks such as updates, large downloads, cryptographic
signing, and file indexing.

**Air** acts as the silent partner to `Mountain`, providing a robust server
environment that persists even when the main editor window is closed, enabling
seamless background updates and persistent state management.

---

## Key Features 🔐

- **Native Sidecar Architecture:** Runs as a standalone process alongside
  `Mountain`, communicating via high-performance IPC (`gRPC`/`Vine`) to handle
  requests without blocking the UI thread.
- **Dedicated Update Management:** Takes full ownership of the update lifecycle
    - downloading, verifying, and applying patches for `Land` - without user
      interruption or restart prompts.
- **File Indexing and Search:** Builds and maintains a comprehensive file index
  with symbol extraction, content analysis, and fast fuzzy search across the
  entire workspace.
- **Isolated Authentication & Signing:** Manages sensitive cryptographic
  operations, including binary signing and secure login flows, keeping security
  logic isolated from the main application view.
- **Background Downloader:** Implements a resilient download manager for
  extensions, language servers, and dependencies, capable of pausing, resuming,
  and handling network interruptions gracefully.
- **Health Monitoring:** Provides multi-level health checks with automatic
  recovery actions, performance tracking, and degradation alerts across all
  daemon services.
- **Resource Offloading:** The designated handler for any "heavy" task that
  doesn't require the main application loop - effectively decoupling
  infrastructure maintenance from the user experience.

---

## Project Structure&#x2001;🗺️

```
Air/
├── Source/
│   ├── Binary.rs                # Library entry point for the Air binary.
│   ├── Library.rs               # Module declarations and crate-level exports.
│   ├── Binary/                  # Daemon process lifecycle (startup, shutdown, monitoring).
│   ├── Daemon/                  # Singleton enforcement, PID locking, platform-native integration.
│   ├── Initialize/              # Configuration, port binding, gRPC server construction, per-service startup.
│   ├── CLI/                     # Command-line interface for daemon interaction and diagnostics.
│   ├── Vine/                    # gRPC protocol implementation (generated proto, server, errors).
│   ├── Mountain/                # gRPC client for communication back to Mountain.
│   ├── ApplicationState/        # Central coordination (connections, service states, telemetry, resources).
│   ├── Configuration/           # TOML config loading with schema validation, env overrides, hot reload.
│   ├── DevLog.rs                # Developer-facing logging and trace ID generation.
│   ├── Updates/                 # Version checking, download, verification, staged install, rollback.
│   ├── Downloader/              # Parallel downloads, chunk transfers, rate limiting, resume capability.
│   ├── Authentication/          # Token management, credential storage, AEAD encryption, key rotation.
│   ├── Indexing/                # File index, symbol extraction, scanning, persistent storage, FS watch.
│   ├── HealthCheck/             # Multi-level health monitoring (alive, responsive, functional).
│   ├── Plugins/                 # Extensible plugin system with sandboxing and lifecycle management.
│   ├── Logging/                 # Structured JSON logging with trace ID propagation and rotation.
│   ├── Metrics/                 # Prometheus-compatible metrics (latency, success rate, resource usage).
│   ├── Resilience/              # Retry with backoff, circuit breaker, bulkhead, timeout management.
│   ├── Security/                # Checksum verification, AES-GCM credential storage, audit subsystem.
│   ├── Tracing/                 # Distributed tracing with sampling, span events, context propagation.
│   └── HTTP/                    # Secure HTTP client with custom DNS, TLS, timeout management.
```

---

## Deep Dive & Component Breakdown🔬

The `Air` daemon is organized into three layers: the binary entry point, the
`Vine` `gRPC` communication layer, and the service modules that perform actual
work. The source structure is documented below with each module's
responsibility.

### Binary and Lifecycle

[`Source/Binary/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Binary/)
orchestrates the daemon process, including startup, shutdown signal handling,
and runtime monitoring.
[`Source/Daemon/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Daemon/)
manages daemon lifecycle with singleton enforcement, `PID` file locking, and
platform-native service integration for graceful shutdown coordination with
`Mountain`.
[`Source/Initialize/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Initialize/)
contains all initialization routines for configuration, port binding, `gRPC`
server construction, and per-service startup including `auth`, `download`,
`echo`, `health` check, `indexing`, `state`, `update`, and `Vine` services.
[`Source/CLI/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/CLI/)
provides the command-line interface for interacting with a running daemon
instance, including command parsing, routing, and diagnostics.

### Communication Layer

[`Source/Vine/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Vine/)
implements the gRPC protocol for communication between Mountain and Air. It
contains the generated protobuf code under `Generated/`, the server
implementation under `Server/`, and error handling types.
[`Source/Mountain/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Mountain/)
provides the gRPC client that Air uses to communicate back to Mountain for
status queries, health checks, and configuration operations.

### State and Configuration

[`Source/ApplicationState/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/ApplicationState/)
acts as the central coordination point, tracking all active client connections,
service states, request telemetry, and system resources with thread-safe async
data structures.
[`Source/Configuration/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Configuration/)
handles configuration loading from TOML files with schema validation,
environment variable overrides, and hot reload support without service restart.
[`Source/DevLog.rs`](https://github.com/CodeEditorLand/Air/tree/Current/Source/DevLog.rs)
provides developer-facing logging utilities and trace ID generation.

### Core Service Modules

[`Source/Updates/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Updates/)
manages the full update lifecycle: version availability checking, secure
download with cryptographic signature verification, multi-checksum integrity
validation, staged installation with atomic application, automatic rollback on
failure, and platform-specific package handling.
[`Source/Downloader/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Downloader/)
provides a resilient download service for extensions, dependencies, and packages
with parallel download support, chunk-based transfer, token-bucket rate
limiting, and resume capability after network interruption.
[`Source/Authentication/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Authentication/)
handles user authentication, token management, and cryptographic operations
including secure credential storage with AEAD encryption and key rotation.
[`Source/Indexing/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Indexing/)
builds and maintains a comprehensive file index with symbol extraction for Rust
and TypeScript, content analysis, file scanning with permission checks, state
management, persistent storage with backup and recovery, and real-time
filesystem watching with debounced event processing.
[`Source/HealthCheck/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/HealthCheck/)
provides multi-level health monitoring (Alive, Responsive, Functional) across
all daemon services with automatic recovery actions, performance indicator
tracking, and degradation alerting.
[`Source/Plugins/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Plugins/)
implements a extensible plugin system with discovery, dynamic loading,
sandboxing, lifecycle management, and inter-plugin communication via message
passing.

### Infrastructure Modules

[`Source/Logging/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Logging/)
provides structured JSON logging with request and trace ID propagation, context
tracking, log rotation by size and time, and sensitive data filtering.
[`Source/Metrics/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Metrics/)
exposes Prometheus-compatible metrics including request latency histograms,
success/failure rates, resource usage counters, and overflow-protected
aggregation.
[`Source/Resilience/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Resilience/)
implements retry with exponential backoff and jitter, circuit breaker for fault
isolation, bulkhead for resource isolation, and timeout management.
[`Source/Security/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Security/)
provides checksum verification, encrypted credential storage with AES-GCM, key
rotation, rate limiting, and a security audit subsystem.
[`Source/Tracing/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Tracing/)
implements distributed tracing with sampling configuration, span event
publishing, trace propagation contexts, and trace metadata management.
[`Source/HTTP/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/HTTP/)
provides a secure HTTP client with custom DNS resolution, TLS configuration, and
timeout management.

---

## System Architecture Diagram 🏗️

```mermaid
graph LR
    classDef mountain fill:#f0d0ff,stroke:#9b59b6,stroke-width:2px,color:#2c0050;
    classDef air      fill:#9cf,stroke:#2471a3,stroke-width:2px,color:#001040;
    classDef external fill:#ebebeb,stroke:#888,stroke-dasharray:5 5,color:#333;

    subgraph "Land Runtime Ecosystem"
        direction TB

        subgraph "⛰️ Mountain - Main App"
            UI["🖼️ User Interface"]:::mountain
            CoreLogic["⚙️ Core Logic"]:::mountain
            IPC_Client["📡 IPC Client"]:::mountain
            CoreLogic -- delegates heavy tasks --> IPC_Client
        end

        subgraph "🪁 Air - Daemon Sidecar (port 50053)"
            IPC_Server["📡 gRPC Server"]:::air
            UpdateMgr["🔄 Update Manager"]:::air
            Downloader["⏬ Resilient Downloader"]:::air
            AuthService["🔐 Signer & Auth"]:::air

            IPC_Server -- routes --> UpdateMgr
            IPC_Server -- routes --> Downloader
            IPC_Server -- routes --> AuthService
        end

        IPC_Client -- Vine gRPC --> IPC_Server
    end

    subgraph "External World"
        Cloud["☁️ Update Servers / Registry"]:::external
    end

    UpdateMgr -- fetches --> Cloud
    Downloader -- downloads --> Cloud
```

---

## `Air`🪁 in the `Land`🏞️ Ecosystem

| Component           | Role & Key Responsibilities                                                                     |
| :------------------ | :---------------------------------------------------------------------------------------------- |
| **Daemon Process**  | Persistent executable that runs independently of the main window, even after the window closes. |
| **Server Host**     | Hosts a local `gRPC` server on `[::1]:50053` to accept commands from `Mountain`.                |
| **Update Delegate** | Sole authority for modifying installation files of the parent application.                      |
| **Signer**          | Handles cryptographic signing of artifacts and secure token storage for user login.             |
| **Traffic Manager** | Proxy/downloader that keeps large network operations off the main renderer process.             |
| **File Indexer**    | Maintains a persistent file index with symbol extraction and fast search across the workspace.  |
| **Health Monitor**  | Periodically checks all service health with automatic recovery and degradation tracking.        |

### Port Allocation

| Process    | Port    | Protocol                    | Purpose                              |
| :--------- | :------ | :-------------------------- | :----------------------------------- |
| **Air**    | `50053` | `Vine`/`Air.proto` (`gRPC`) | Daemon services - updates, downloads |
| **Cocoon** | `50052` | `Vine.proto` (`gRPC`)       | `VS Code` extension hosting          |

---

## Getting Started 🚀

### Installation 📥

To add `Air` to your project workspace:

```toml
[dependencies]
Air = { git = "https://github.com/CodeEditorLand/Air.git", branch = "Current" }
```

### Usage Pattern 🚀

**Air** is typically spawned automatically by `Mountain` during startup.

1. **Spawn:** `Mountain` detects if `Air` is running. If not, it spawns the
   binary.
2. **Connect:** `Mountain` establishes a `Vine` (`gRPC`) connection to `Air`'s
   local port `[::1]:50053`.
3. **Delegate:** When a user requests an update or large download, `Mountain`
   sends a command to `Air` and immediately returns control to the user.
4. **Monitor:** `Air` emits progress events back to `Mountain` to update the UI
   status bars.

---

## See Also 🔗

- [`Mountain`](https://github.com/CodeEditorLand/Mountain)
- [`Vine`](https://github.com/CodeEditorLand/Vine)
- [`Echo`](https://github.com/CodeEditorLand/Echo)
- [`Mist`](https://github.com/CodeEditorLand/Mist)

---

## License ⚖️

This project is released into the public domain under the **Creative Commons CC0
Universal** license. You are free to use, modify, distribute, and build upon
this work for any purpose, without any restrictions. For the full legal text,
see the [`LICENSE`](https://github.com/CodeEditorLand/Air/tree/Current/) file.

---

## Changelog 📜

See [`CHANGELOG.md`](https://github.com/CodeEditorLand/Air/tree/Current/) for a
history of changes specific to **Air**🪁.

---

## Funding \& Acknowledgements 🙏🏻

**Air**🪁 is a core element of the **Land**🏞️ ecosystem. This project is funded
through [NGI0 Commons Fund](https://NLnet.NL/commonsfund), a fund established by
[NLnet](https://NLnet.NL) with financial support from the European Commission's
[Next Generation Internet](https://ngi.eu) program. Learn more at the
[NLnet project page](https://NLnet.NL/project/Land).

The project is operated by PlayForm, based in Sofia, Bulgaria.

PlayForm acts as the open-source steward for Code Editor Land under the NGI0
Commons Fund grant.

<table>
	<thead>
		<tr>
			<th align="left"><strong>Land</strong></th>
			<th align="left"><strong>PlayForm</strong></th>
			<th align="left"><strong>NLnet</strong></th>
			<th align="left"><strong>NGI0 Commons Fund</strong></th>
		</tr>
	</thead>
	<tbody>
		<tr>
			<td align="left" valign="middle">
				<a href="https://Editor.Land">
					<img width="60" src="https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Land.svg" alt="Land">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://PlayForm.Cloud">
					<img width="76" src="https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg" alt="PlayForm">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL">
					<img width="240" src="https://NLnet.NL/logo/banner.svg" alt="NLnet">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL/commonsfund">
					<img width="240" src="https://NLnet.NL/image/logos/NGI0CommonsFund_tag_black_mono.svg" alt="NGI0 Commons Fund">
				</a>
			</td>
		</tr>
	</tbody>
</table>

---

**Project Maintainers**: Source Open (Source/Open@Editor.Land) |
[GitHub Repository](https://github.com/CodeEditorLand/Air) |
[Report an Issue](https://github.com/CodeEditorLand/Air/issues) |
[Security Policy](https://github.com/CodeEditorLand/Air/security/policy)
