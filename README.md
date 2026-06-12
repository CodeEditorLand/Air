# **Air**&#x2001;🪁

<table>
	<tr>
		<td>
			<a href="https://GitHub.Com/CodeEditorLand/Air" target="_blank">
				<picture>
					<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/last-commit/CodeEditorLand/Air?label=Last-commit&color=black&labelColor=black&logoColor=white&logoWidth=0" />
					<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/last-commit/CodeEditorLand/Air?label=Last-commit&color=white&labelColor=white&logoColor=black&logoWidth=0" />
					<img src="https://img.shields.io/github/last-commit/CodeEditorLand/Air?label=Last-commit&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Last-commit" title="Last-commit" />
				</picture>
			</a>
			<br />
			<a href="https://GitHub.Com/CodeEditorLand/Air" target="_blank">
				<picture>
					<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/issues/CodeEditorLand/Air?label=Issues&color=black&labelColor=black&logoColor=white&logoWidth=0" />
					<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/issues/CodeEditorLand/Air?label=Issues&color=white&labelColor=white&logoColor=black&logoWidth=0" />
					<img src="https://img.shields.io/github/issues/CodeEditorLand/Air?label=Issues&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Issues" title="Issues" />
				</picture>
			</a>
		</td>
		<td>
			<a href="https://github.com/CodeEditorLand/Air" target="_blank">
				<picture>
					<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/stars/CodeEditorLand/Air?style=flat&label=Star&logo=github&color=black&labelColor=black&logoColor=white&logoWidth=0" />
					<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/stars/CodeEditorLand/Air?style=flat&label=Star&logo=github&color=white&labelColor=white&logoColor=black&logoWidth=0" />
					<img src="https://img.shields.io/github/stars/CodeEditorLand/Air?style=flat&label=Star&logo=github&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Star" />
				</picture>
			</a>
			<br />
			<a href="https://GitHub.Com/CodeEditorLand/Air" target="_blank">
				<picture>
					<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/downloads/CodeEditorLand/Air?label=Downloads&color=black&labelColor=black&logoColor=white&logoWidth=0" />
					<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/downloads/CodeEditorLand/Air?label=Downloads&color=white&labelColor=white&logoColor=black&logoWidth=0" />
					<img src="https://img.shields.io/github/downloads/CodeEditorLand/Air?label=Downloads&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Downloads" title="Downloads" />
				</picture>
			</a>
		</td>
	</tr>
</table>

The Native Background Daemon for `Land`&#x2001;🏞️

> **`VS Code` cold-starts slowly because everything initializes fresh each
> launch. Updates require a full restart that kills open terminals and
> in-progress work. There is no mechanism to pre-stage work between sessions.**
>
> _\"The next version is already downloaded and verified before you decide to
> update. The main window never blocks waiting for a download.\"_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](https://github.com/CodeEditorLand/Air/tree/Current/LICENSE)

**[Rust API Documentation](https://rust.documentation.air.editor.land/)**&#x2001;📖

---

## Overview

**Air** is the lightweight, persistent daemon that powers the background
capabilities of the **Land** Code Editor. While **Mountain** handles the core
application logic and UI, **Air** operates as a specialized sidecar process
dedicated to heavy lifting, network operations, and system maintenance. It
ensures that the main editor remains responsive by offloading resource-intensive
tasks such as updates, large downloads, cryptographic signing, and file
indexing.

**Air** is engineered to:

1. **Serve as the Persistent Background Daemon:** Run as a standalone process
   alongside **Mountain**, surviving window closures and maintaining background
   services across sessions.
2. **Own the Update Lifecycle:** Take full ownership of downloading, verifying,
   and applying patches for **Land** without user interruption or restart
   prompts.
3. **Offload Heavy Network Operations:** Act as the traffic manager for large
   downloads (extensions, language servers, dependencies) with resilient
   resume-capable transfers.
4. **Isolate Security-Critical Operations:** Manage cryptographic signing,
   secure credential storage, and authentication token lifecycle, keeping
   sensitive logic isolated from the main application view.
5. **Maintain the File Index:** Build and persist a comprehensive file index
   with symbol extraction and fast fuzzy search across the entire workspace.

---

## Key Features&#x2001;🪁

**`gRPC` Native Communication** — All inter-process communication with
**Mountain** travels over a **Vine** (`tonic`-based `gRPC`) channel on
`[::1]:50053`, providing strongly-typed `protobuf` contracts, bi-directional
streaming for progress events, and a well-defined API surface generated from
`Air.proto`.

**Self-Contained Daemon Lifecycle** — Runs as an independent process with
singleton enforcement via PID locking, graceful shutdown on `SIGTERM`, and
platform-native daemonization on `macOS`, `Linux`, and `Windows`. Survives
window closures and persists across editor sessions.

**Resilient Update Engine** — Full ownership of the update lifecycle: version
checking against multiple channels (stable, beta, nightly), concurrent chunked
downloads with resume capability, cryptographic checksum verification, staged
installation, and automatic rollback on failure.

**Isolated Security Boundaries** — Cryptographic signing with `ring`, AEAD
encrypted credential storage with `zeroize`, token lifecycle management, rate
limiting via token bucket, and a comprehensive security audit subsystem — all
isolated from the main application process.

**Real-Time File Indexing** — Persistent file index with `Rust` and
`TypeScript` symbol extraction, recursive directory scanning, `notify`-based
file system watching for live updates, and a fast query engine with fuzzy
search across the entire workspace.

**Observability by Default** — Structured `JSON` logging with trace-ID
propagation, `OpenTelemetry`-compatible distributed tracing with configurable
sampling, and `Prometheus`-compatible metrics for latency, success rates, and
resource utilization across every service.

**Resilience Everywhere** — All network operations are wrapped in retry-with-
exponential-backoff, circuit breakers with half-open probing, bulkhead
executors for service isolation, and configurable timeouts via the shared
`Resilience/` infrastructure.

---

## Core Architecture Principles&#x2001;🏗️

| Principle | Description | Key Components |
|-----------|-------------|----------------|
| **Sidecar Isolation** | Run as a standalone daemon process, surviving independently of the main window lifecycle for persistent background operations. | `Daemon/`, `Binary/`, PID locking |
| **`gRPC` IPC Boundary** | Use **Vine** (`tonic`-based `gRPC`) for all communication with **Mountain**, ensuring a high-performance and well-defined API. | `Vine/`, `Air.proto`, generated `prost` bindings |
| **Service Modularity** | Each capability (updates, downloads, auth, indexing) lives in its own module with independent startup and health monitoring. | `Updates/`, `Downloader/`, `Authentication/`, `Indexing/` |
| **Resilience by Default** | Wrap all network operations in retry-with-backoff, circuit breakers, bulkheads, and timeouts via the shared `Resilience/` library. | `Resilience/`, `HealthCheck/` |
| **Declarative Configuration** | Load `TOML` config with schema validation, environment overrides, and hot reload without service interruption. | `Configuration/`, `Initialize/` |
| **Observable Operations** | Emit structured `JSON` logs, distributed traces, and `Prometheus` metrics for every delegated task. | `Logging/`, `Tracing/`, `Metrics/` |
| **Secure Credential Handling** | Never expose raw secrets; store credentials with AEAD encryption (`ring`), enforce key rotation, and audit all access. | `Security/`, `Authentication/`, `zeroize` |

---

## System Architecture&#x2001;

```mermaid
graph LR
    classDef mountain fill:#f0d0ff,stroke:#9b59b6,stroke-width:2px,color:#2c0050;
    classDef air      fill:#e0f4ff,stroke:#2471a3,stroke-width:2px,color:#001040;
    classDef external fill:#ebebeb,stroke:#888,stroke-width:1px,stroke-dasharray:5 5,color:#333;
    classDef infra    fill:#fff3c0,stroke:#f39c12,stroke-width:1px,stroke-dasharray:5 5,color:#5a3e00;

    subgraph MOUNTAIN[\"Mountain ⛰️ - Main Application\"]
        MountainIPC[\"Mountain gRPC client&#x2001;delegates heavy tasks\"]:::mountain
    end

    subgraph AIR[\"Air 🪁 - Persistent Background Daemon (::1:50053)\"]
        direction TB
        subgraph COMM[\"Vine/ - gRPC Transport\"]
            VineServer[\"Vine/Server/ - gRPC server&#x2001;(Generated/ prost bindings)\"]:::air
            MountainClient[\"Mountain gRPC client&#x2001;(Air → Mountain callbacks)\"]:::air
        end
        subgraph CORE[\"Core Services\"]
            Updates[\"Updates/ - version check&#x2001;download · verify · staged install · rollback\"]:::air
            Downloader[\"Downloader/ - parallel chunks&#x2001;rate-limit · resume · retry\"]:::air
            Auth[\"Authentication/ - token mgmt&#x2001;AEAD encrypt · key rotation\"]:::air
            Indexing[\"Indexing/ - file index&#x2001;symbol extract · FS watch · search\"]:::air
        end
        subgraph INFRA[\"Infrastructure\"]
            Health[\"HealthCheck/ - Alive/Responsive/Functional&#x2001;auto-recovery\"]:::infra
            Resilience[\"Resilience/ - retry backoff&#x2001;circuit breaker · bulkhead\"]:::infra
            Metrics[\"Metrics/ - Prometheus-compatible&#x2001;latency · success rate\"]:::infra
            Security[\"Security/ - AES-GCM&#x2001;checksum · audit\"]:::infra
            Daemon[\"Daemon/ - PID lock&#x2001;singleton enforce\"]:::air
        end

        VineServer --> Updates
        VineServer --> Downloader
        VineServer --> Auth
        VineServer --> Indexing
        Updates --> Resilience
        Downloader --> Resilience
    end

    subgraph EXTERNAL[\"External ☁️\"]
        UpdateSrv[\"Update servers / extension registry\"]:::external
    end

    MountainIPC -- gRPC :50053 --> VineServer
    MountainClient -- progress events --> MountainIPC
    Updates -- fetches --> UpdateSrv
    Downloader -- downloads --> UpdateSrv
```

**Connection paths:**

| Path | Protocol | Use Case |
|------|----------|----------|
| **Mountain** → **Air** via `gRPC` | `protobuf` over `gRPC` on port 50053 | Delegate updates, downloads, indexing, auth |
| **Air** → **Mountain** via `gRPC` callback | `protobuf` over `gRPC` | Progress events, health status, metrics |
| **Air** → External via `HTTP` | `HTTPS` with `Mist` DNS isolation | Update servers, extension registries |

---

## Key Components

| Component | Path | Description |
|-----------|------|-------------|
| Binary Entry Point | `Source/Binary.rs` | Binary entry point for the Air daemon. |
| Library Root | `Source/Library.rs` | Module declarations and crate-level exports. |
| Daemon Lifecycle | `Source/Binary/` | Daemon process lifecycle (startup, shutdown, monitoring). |
| Singleton Enforcer | `Source/Daemon/` | Singleton enforcement, PID locking, platform-native integration. |
| Initialization | `Source/Initialize/` | Configuration, port binding, `gRPC` server construction, per-service startup. |
| CLI | `Source/CLI/` | Command-line interface for daemon interaction and diagnostics. |
| `gRPC` Client | `Source/Client/` | Typed `gRPC` client (`AirClient`) and service provider (`AirServiceProvider`) for **Mountain** interaction. |
| `gRPC` Protocol | `Source/Vine/` | `gRPC` protocol implementation (generated `prost` bindings, server, errors). |
| Application State | `Source/ApplicationState/` | Central coordination (connections, service states, telemetry, resources). |
| Configuration | `Source/Configuration/` | `TOML` config loading with schema validation, env overrides, hot reload. |
| Updates | `Source/Updates/` | Version checking, download, verification, staged install, rollback. |
| Downloader | `Source/Downloader/` | Parallel downloads, chunk transfers, rate limiting, resume capability. |
| Authentication | `Source/Authentication/` | Token management, credential storage, AEAD encryption, key rotation. |
| Indexing | `Source/Indexing/` | File index, symbol extraction, scanning, persistent storage, FS watch. |
| Health Check | `Source/HealthCheck/` | Multi-level health monitoring (alive, responsive, functional) with auto-recovery. |
| Logging | `Source/Logging/` | Structured `JSON` logging with trace-ID propagation, rotation, sensitive data filtering. |
| Metrics | `Source/Metrics/` | `Prometheus`-compatible metrics (latency, success rate, resource usage). |
| Resilience | `Source/Resilience/` | Retry with exponential backoff, circuit breaker, bulkhead, timeout management. |
| Security | `Source/Security/` | Checksum verification, AES-GCM credential storage, rate limiting, audit subsystem. |
| Tracing | `Source/Tracing/` | Distributed tracing with sampling, span events, context propagation. |
| HTTP Client | `Source/HTTP/` | Secure HTTP client with custom DNS via `Mist`, `TLS`, timeout management. |
| Mountain Bridge | `Source/Mountain/` | Client for **Mountain** callbacks with `TLS` configuration. |
| Plugins | `Source/Plugins/` | Plugin API versioning and event bus for extensibility. |

---

## Project Structure&#x2001;🗺️

```
Element/Air/
├── Cargo.toml                    # Package manifest with feature flags
├── build.rs                      # Build script (tonic/prost codegen)
├── LICENSE                       # CC0-1.0 license
├── Source/
│   ├── Binary.rs                 # Binary entry point
│   ├── Library.rs                # Library root (rlib)
│   ├── DevLog.rs                 # Development logging utilities
│   ├── ApplicationState/
│   │   └── mod.rs                # Central state coordination
│   ├── Authentication/
│   │   └── mod.rs                # Token and credential management
│   ├── Binary/
│   │   ├── mod.rs
│   │   ├── Binary.rs             # Binary initialization
│   │   ├── Monitor/
│   │   │   └── StartMonitoring.rs # Process monitoring
│   │   └── Shutdown/
│   │       └── WaitForShutdownSignal.rs # Graceful shutdown
│   ├── CLI/
│   │   ├── mod.rs
│   │   ├── CliHandler.rs         # CLI command handler
│   │   ├── CliParser.rs          # Argument parser
│   │   ├── CommandTypes.rs       # Command type definitions
│   │   ├── DaemonClient.rs       # Daemon CLI client
│   │   ├── OutputFormat.rs       # Output format types
│   │   ├── OutputFormatter.rs    # Output formatting
│   │   ├── ResponseTypes.rs      # Response type definitions
│   │   └── Tests.rs              # CLI integration tests
│   ├── Client/
│   │   ├── mod.rs
│   │   ├── AirClient/            # Typed client methods
│   │   │   ├── mod.rs
│   │   │   ├── ApplyUpdate.rs
│   │   │   ├── Authenticate.rs
│   │   │   ├── CheckForUpdates.rs
│   │   │   ├── DownloadFile.rs
│   │   │   ├── DownloadStream.rs
│   │   │   ├── DownloadUpdate.rs
│   │   │   ├── GetConfiguration.rs
│   │   │   ├── GetFileInfo.rs
│   │   │   ├── GetMetrics.rs
│   │   │   ├── GetResourceUsage.rs
│   │   │   ├── GetStatus.rs
│   │   │   ├── HealthCheck.rs
│   │   │   ├── IndexFiles.rs
│   │   │   ├── SearchFiles.rs
│   │   │   ├── SetResourceLimits.rs
│   │   │   ├── UpdateConfiguration.rs
│   │   │   └── (supporting types)
│   │   └── AirServiceProvider/   # Service provider implementations
│   │       ├── mod.rs
│   │       ├── ApplyUpdate.rs
│   │       ├── Authenticate.rs
│   │       ├── CheckForUpdates.rs
│   │       ├── DownloadFile.rs
│   │       ├── DownloadStream.rs
│   │       ├── DownloadUpdate.rs
│   │       ├── GenerateRequestID.rs
│   │       ├── GetConfiguration.rs
│   │       ├── GetFileInfo.rs
│   │       ├── GetMetrics.rs
│   │       ├── GetResourceUsage.rs
│   │       ├── GetStatus.rs
│   │       ├── HealthCheck.rs
│   │       ├── IndexFiles.rs
│   │       ├── SearchFiles.rs
│   │       ├── SetResourceLimits.rs
│   │       └── UpdateConfiguration.rs
│   ├── Configuration/
│   │   ├── mod.rs
│   │   ├── AirConfiguration.rs   # Main configuration struct
│   │   ├── ConfigurationManager.rs # Config loading and management
│   │   ├── HotReload.rs          # Hot-reload support
│   │   ├── Schema.rs             # Schema validation
│   │   └── Tests.rs              # Configuration tests
│   ├── Daemon/
│   │   ├── mod.rs
│   │   ├── DaemonManager.rs      # Daemon lifecycle manager
│   │   ├── DaemonStatus.rs       # Daemon status types
│   │   ├── ExitCode.rs           # Exit code definitions
│   │   ├── Platform.rs           # Platform abstraction
│   │   └── PlatformInfo.rs       # Platform information
│   ├── Downloader/
│   │   ├── mod.rs
│   │   ├── RateLimit.rs          # Download rate limiting
│   │   └── Types.rs              # Downloader type definitions
│   ├── HTTP/
│   │   ├── mod.rs
│   │   └── Client.rs             # Secure HTTP client
│   ├── HealthCheck/
│   │   ├── mod.rs
│   │   ├── DegradationLevel.rs   # Service degradation levels
│   │   ├── HealthCheckConfig.rs  # Health check configuration
│   │   ├── HealthCheckLevel.rs   # Check depth levels
│   │   ├── HealthCheckManager.rs # Health check orchestrator
│   │   ├── HealthCheckRecord.rs  # Check result records
│   │   ├── HealthCheckResponse.rs # Health check response
│   │   ├── HealthStatistics.rs   # Health statistics aggregation
│   │   ├── HealthStatus.rs       # Health status enum
│   │   ├── PerformanceIndicators.rs # Performance metrics
│   │   ├── RecoveryAction.rs     # Recovery action definitions
│   │   ├── RecoveryActionType.rs # Recovery action types
│   │   ├── RecoveryTrigger.rs    # Recovery trigger logic
│   │   ├── ResourceWarning.rs    # Resource warning types
│   │   ├── ResourceWarningType.rs # Warning type enum
│   │   ├── ServiceHealth.rs      # Per-service health tracking
│   │   └── WarningSeverity.rs    # Warning severity levels
│   ├── Indexing/
│   │   ├── mod.rs
│   │   ├── Background/
│   │   │   ├── mod.rs
│   │   │   └── StartWatcher.rs   # Background file watcher
│   │   ├── Language/
│   │   │   ├── mod.rs
│   │   │   ├── ParseRust.rs      # Rust symbol extraction
│   │   │   └── ParseTypeScript.rs # TypeScript symbol extraction
│   │   ├── Process/
│   │   │   ├── mod.rs
│   │   │   ├── ExtractSymbols.rs # Symbol extraction engine
│   │   │   └── ProcessContent.rs # Content processing
│   │   ├── Scan/
│   │   │   ├── mod.rs
│   │   │   ├── ScanDirectory.rs  # Directory scanner
│   │   │   └── ScanFile.rs       # Single file scanner
│   │   ├── State/
│   │   │   ├── mod.rs
│   │   │   ├── CreateState.rs    # Index state creation
│   │   │   └── UpdateState.rs    # Index state updates
│   │   ├── Store/
│   │   │   ├── mod.rs
│   │   │   ├── QueryIndex.rs     # Index query engine
│   │   │   ├── StoreEntry.rs     # Index entry storage
│   │   │   └── UpdateIndex.rs    # Index update operations
│   │   └── Watch/
│   │       ├── mod.rs
│   │       └── WatchFile.rs      # File change watcher
│   ├── Initialize/
│   │   ├── mod.rs
│   │   ├── Build/
│   │   │   ├── mod.rs
│   │   │   └── BuildServer.rs    # gRPC server construction
│   │   ├── Command/
│   │   │   ├── mod.rs
│   │   │   ├── Connect/
│   │   │   │   ├── mod.rs
│   │   │   │   └── ConnectDaemon.rs # Daemon connection
│   │   │   ├── HandleCommand.rs  # Command dispatcher
│   │   │   ├── ParseArguments.rs # Argument parsing
│   │   │   └── ValidateCommand.rs # Command validation
│   │   ├── Configure/
│   │   │   ├── mod.rs
│   │   │   ├── Log/
│   │   │   │   └── ConfigureLog.rs  # Log configuration
│   │   │   └── Port/
│   │   │       └── SelectPort.rs # Port selection
│   │   └── Service/
│   │       ├── mod.rs
│   │       ├── Auth/
│   │       │   └── StartAuth.rs  # Auth service startup
│   │       ├── Download/
│   │       │   └── StartDownload.rs # Download service startup
│   │       ├── Echo/
│   │       │   └── StartEcho.rs  # Echo service startup
│   │       ├── Health/
│   │       │   └── StartHealthCheck.rs # Health check startup
│   │       ├── Index/
│   │       │   └── StartIndex.rs # Indexing service startup
│   │       ├── State/
│   │       │   └── CreateState.rs # Application state creation
│   │       ├── Update/
│   │       │   └── StartUpdate.rs # Update service startup
│   │       └── Vine/
│   │           └── StartService.rs # gRPC service startup
│   ├── Logging/
│   │   ├── mod.rs
│   │   ├── ContextLogger.rs      # Context-aware logger
│   │   ├── LogContext.rs         # Log context types
│   │   ├── LogManager.rs         # Log manager
│   │   ├── LogRotationConfig.rs  # Log rotation configuration
│   │   ├── SensitiveDataConfig.rs # Sensitive data config
│   │   ├── SensitiveDataFilter.rs # Sensitive data redaction
│   │   └── StructuredLogEntry.rs # Structured log entry format
│   ├── Metrics/
│   │   └── mod.rs                # Prometheus metrics collection
│   ├── Mountain/
│   │   ├── mod.rs
│   │   ├── Constants.rs          # Mountain connection constants
│   │   ├── MountainClient.rs     # Mountain gRPC client
│   │   ├── MountainClientConfig.rs # Mountain client configuration
│   │   └── TlsConfig.rs          # TLS configuration
│   ├── Plugins/
│   │   ├── mod.rs
│   │   ├── ApiVersion.rs         # Plugin API versioning
│   │   └── EventBus.rs           # Plugin event bus
│   ├── Resilience/
│   │   ├── mod.rs
│   │   ├── BulkheadConfig.rs     # Bulkhead configuration
│   │   ├── BulkheadExecutor.rs   # Bulkhead execution
│   │   ├── BulkheadStatistics.rs # Bulkhead metrics
│   │   ├── CircuitBreaker.rs     # Circuit breaker state machine
│   │   ├── CircuitBreakerConfig.rs # Circuit breaker config
│   │   ├── CircuitEvent.rs       # Circuit events
│   │   ├── CircuitState.rs       # Circuit state enum
│   │   ├── CircuitStatistics.rs  # Circuit metrics
│   │   ├── ResilienceOrchestrator.rs # Resilience orchestrator
│   │   ├── ResilienceTests.rs    # Resilience test suite
│   │   ├── Retry.rs              # Retry with exponential backoff
│   │   └── Timeout.rs            # Timeout configuration
│   ├── Security/
│   │   ├── mod.rs
│   │   ├── ChecksumVerifier.rs   # Checksum verification
│   │   ├── RateLimitConfig.rs    # Rate limit configuration
│   │   ├── RateLimiter.rs        # Rate limiter implementation
│   │   ├── RateLimitStatus.rs    # Rate limit status
│   │   ├── SecureBytes.rs        # Secure byte handling
│   │   ├── SecureStorage.rs      # AEAD encrypted storage
│   │   ├── SecurityAuditor.rs    # Security audit subsystem
│   │   ├── SecurityEvent.rs      # Security event types
│   │   ├── SecurityEventType.rs  # Security event enum
│   │   ├── SecuritySeverity.rs   # Severity levels
│   │   ├── SecurityTests.rs      # Security test suite
│   │   └── TokenBucket.rs        # Token bucket algorithm
│   ├── Tracing/
│   │   └── mod.rs                # Distributed tracing
│   ├── Updates/
│   │   ├── mod.rs
│   │   ├── ChecksumUtil.rs       # Update checksum utilities
│   │   ├── DownloadSession.rs    # Download session management
│   │   ├── InstallationStatus.rs # Installation status tracking
│   │   ├── PackageFormat.rs      # Package format support
│   │   ├── PlatformConfig.rs     # Platform-specific config
│   │   ├── PlatformDetect.rs     # Platform detection
│   │   ├── PlatformMetadata.rs   # Platform metadata
│   │   ├── RollbackHistory.rs    # Rollback history tracking
│   │   ├── RollbackState.rs      # Rollback state machine
│   │   ├── Types.rs              # Update type definitions
│   │   ├── UpdateChannel.rs      # Update channel enum
│   │   ├── UpdateInfo.rs         # Update information
│   │   ├── UpdateManager.rs      # Update lifecycle manager
│   │   ├── UpdateStatus.rs       # Update status types
│   │   ├── UpdateTelemetry.rs    # Update telemetry
│   │   └── VersionCompare.rs     # Semantic version comparison
│   └── Vine/
│       ├── mod.rs
│       ├── Error.rs              # gRPC error types
│       ├── Generated/
│       │   ├── mod.rs
│       │   └── air.rs            # Generated prost bindings
│       └── Server/
│           ├── mod.rs
│           └── AirVinegRPCService.rs # gRPC service implementation
└── Documentation/
    ├── GitHub/
    │   ├── Architecture.md       # Internal module architecture
    │   └── DeepDive.md           # In-depth technical details
    └── Rust/
        └── doc/                  # Cargo doc output
```

---

## In the Land Project

**Air** is the persistent background daemon for the Land ecosystem. It
communicates with **Mountain** via **Vine** (`gRPC`) on port `[::1]:50053` and
uses **Mist** for DNS isolation on its HTTP client.

| Role | Details |
|------|---------|
| **Daemon Process** | Persistent executable that runs independently of the main window, even after the window closes. |
| **Server Host** | Hosts a local `gRPC` server on `[::1]:50053` to accept commands from **Mountain**. |
| **Update Delegate** | Sole authority for modifying installation files of the parent application. |
| **Signer** | Handles cryptographic signing of artifacts and secure token storage for user login. |
| **Traffic Manager** | Proxy/downloader that keeps large network operations off the main renderer process. |
| **File Indexer** | Maintains a persistent file index with symbol extraction and fast search across the workspace. |
| **Health Monitor** | Periodically checks all service health with automatic recovery and degradation tracking. |

### Port Allocation

| Process | Port | Protocol | Purpose |
|---------|------|----------|---------|
| **Air** | `50053` | **Vine**/`Air.proto` (`gRPC`) | Daemon services — updates, downloads, indexing |
| **Cocoon** | `50052` | `Vine.proto` (`gRPC`) | VS Code extension hosting |

**Air** is part of the networking/IPC connectivity stack alongside **Mist** 🌫️
(DNS isolation) and **Vine** 🌿 (`gRPC` protocol layer).

Typical usage flow:

1. **Spawn:** **Mountain** detects if **Air** is running. If not, it spawns the
   binary.
2. **Connect:** **Mountain** establishes a **Vine** (`gRPC`) connection to
   **Air**'s local port `[::1]:50053`.
3. **Delegate:** When a user requests an update or large download, **Mountain**
   sends a command to **Air** and immediately returns control to the user.
4. **Monitor:** **Air** emits progress events back to **Mountain** to update the
   UI status bars.

---

## Getting Started&#x2001;🚀

### Prerequisites

- `Rust` 1.75 or later
- Protocol Buffer compiler (included via `tonic-build` build dependency)

### Build

```bash
cd Element/Air
cargo build --release
```

### Run

```bash
# Run with default settings
./Target/release/Air

# Or via cargo
cargo run --bin Air
```

### Available Features

| Feature | Description |
|---------|-------------|
| `default` | Enables `full-services` and `mtls` |
| `full-services` | Enables `authentication`, `updates`, `downloader`, `indexing` |
| `authentication` | Token management and credential storage |
| `updates` | Update lifecycle management |
| `downloader` | Parallel chunked downloads with resume |
| `indexing` | File indexing with symbol extraction |
| `mtls` | Mutual `TLS` for `gRPC` connections |
| `appimage` | `AppImage` package format support |
| `deb` | `Debian` package format support |
| `rpm` | `RPM` package format support |

### Build with Features

```bash
# Default features (full-services + mTLS)
cargo build --release

# Minimal daemon (no update/auth/indexing)
cargo build --release --no-default-features

# All features
cargo build --release --all-features
```

### Key Dependencies

| Crate / Package | Purpose |
|-----------------|---------|
| `tonic` / `prost` | `gRPC` server and Protocol Buffer code generation |
| **Vine** | Local path dependency — generated `Air.proto` `gRPC` contracts |
| `Common` | Local path dependency — shared types and abstractions |
| **Mist** | Local path dependency — DNS isolation for HTTP client |
| `reqwest` / `rustls` | `HTTPS` downloads with `TLS` certificate verification |
| `tokio` | Async runtime for concurrent I/O and task scheduling |
| `notify` / `ignore` | File system event watching for real-time index updates |
| `ring` / `zeroize` | Cryptographic signing and secure credential storage |
| `tracing` | Structured `JSON` logging with span propagation |
| `config` / `toml` | Configuration file loading with hot-reload support |
| `sysinfo` / `systemstat` | System resource monitoring and health checks |
| `walkdir` / `ignore` | Recursive directory traversal for file indexing |

---

## Security&#x2001;🔒

**Air** enforces security at multiple layers, isolating sensitive operations
from the main application process:

| Layer | Mechanism |
|-------|-----------|
| **Process Isolation** | Separate daemon process — cryptographic and auth logic never runs in the renderer |
| **Network** | `mTLS` for `gRPC` connections, `Mist` DNS isolation for outbound HTTP |
| **Credentials** | AEAD encryption via `ring`, `zeroize`-protected memory, key rotation |
| **Rate Limiting** | Token bucket algorithm per-endpoint rate limiting |
| **Checksum Verification** | All downloaded artifacts verified via `SHA-256` / `MD5` before installation |
| **Audit Logging** | Security audit subsystem with severity-classified events |
| **Singleton Enforcement** | PID locking prevents duplicate daemon instances |

---

## API Reference

- **[Rust API Documentation](https://rust.documentation.air.editor.land/)**&#x2001;📖
- [Deep Dive](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/DeepDive.md) — Detailed startup sequence, `gRPC` routing, and data flow

---

## Related Documentation

- [Architecture Overview](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/Architecture.md) — Internal module structure
- [Deep Dive](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/DeepDive.md) — In-depth technical details
- [Land Documentation](../../Documentation/GitHub/README.md) — Complete documentation index
- **Mountain** ⛰️ — Main application process — [GitHub](https://github.com/CodeEditorLand/Mountain)
- **Mist** 🌫️ — DNS isolation for the private network — [GitHub](https://github.com/CodeEditorLand/Mist)
- **Vine** 🌿 — `gRPC` protocol layer — [GitHub](https://github.com/CodeEditorLand/Vine)
- **Cocoon** 🦋 — `Node.js`/`Effect-TS` extension host — [GitHub](https://github.com/CodeEditorLand/Cocoon)
- **Grove** 🌳 — `Rust`/`WASM` extension host — [GitHub](https://github.com/CodeEditorLand/Grove)
- **Echo** 📣 — [GitHub](https://github.com/CodeEditorLand/Echo)
- **Common** — Shared types and abstractions — [GitHub](https://github.com/CodeEditorLand/Common)

---

## Funding & Acknowledgements&#x2001;🙏🏻

This project is funded through
[NGI0 Commons Fund](https://NLnet.NL/commonsfund), a fund established by
[NLnet](https://NLnet.NL) with financial support from the European Commission's
Next Generation Internet program, under grant agreement No 101135429.

The project is operated by PlayForm, based in Sofia, Bulgaria. PlayForm acts as
the open-source steward for Code Editor Land under the NGI0 Commons Fund grant.

<table>
	<tbody>
		<tr>
			<td align="left" valign="middle">
				<a href="https://Editor.Land">
					<img width="60" src="https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Land.svg" alt="Land" />
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://PlayForm.Cloud">
					<img width="76" src="https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg" alt="PlayForm" />
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL">
					<img width="240" src="https://NLnet.NL/logo/banner.svg" alt="NLnet" />
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL/commonsfund">
					<img width="240" src="https://NLnet.NL/image/logos/NGI0CommonsFund_tag_black_mono.svg" alt="NGI0 Commons Fund" />
				</a>
			</td>
		</tr>
	</tbody>
</table>

---

**Project Maintainers**: Source Open (Source/Open@editor.land) |
[GitHub Repository](https://github.com/CodeEditorLand/Air) |
[Report an Issue](https://github.com/CodeEditorLand/Air/issues) |
[Security Policy](https://github.com/CodeEditorLand/Air/security/policy)
