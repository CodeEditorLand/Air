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
					<img src="https://img.shields.io/github/stars/CodeEditorLand/Air?style=flat&label=Star&logo=github&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Star" title="Star" />
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

_"The next version is already downloaded and verified before you decide to
update. The main window never blocks waiting for a download."_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](https://github.com/CodeEditorLand/Air/tree/Current/LICENSE)
[<img src="https://editor.land/Image/Rust.svg" width="14" alt="Rust" />](https://www.rust-lang.org/)&#x2001;[![Crates.io](https://img.shields.io/crates/v/Air.svg)](https://crates.io/crates/Air)
[<img src="https://editor.land/Image/Rust.svg" width="14" alt="Rust" />](https://www.rust-lang.org/)&#x2001;[![Rust Version](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

**[Rust API Documentation](https://rust.documentation.air.editor.land/)**&#x2001;📖

---

## Overview

**Air** is the lightweight, persistent daemon that powers the background
capabilities of the **Land** Code Editor. While **Mountain** ⛰️ handles the core
application logic and UI, **Air** operates as a specialized sidecar process
dedicated to heavy lifting, network operations, and system maintenance. It
ensures that the main editor remains responsive by offloading resource-intensive
tasks such as updates, large downloads, cryptographic signing, and file
indexing.

`VS Code` cold-starts slowly because everything initializes fresh each launch,
and updates require a full restart. **Air** solves this by running as a
persistent background process that survives window closures, pre-stages updates,
and keeps a warm file index across sessions - so the editor is always ready the
moment you launch it.

**Air is engineered to:**

1. **Serve as the Persistent Background Daemon** - Run as a standalone process
   alongside **Mountain** ⛰️, surviving window closures and maintaining
   background services across sessions via `Daemon/` singleton enforcement and
   platform-native daemonization.
2. **Own the Update Lifecycle** - Take full ownership of downloading, verifying,
   and applying patches for **Land** without user interruption or restart
   prompts, with staged installation and automatic rollback via `Updates/`.
3. **Offload Heavy Network Operations** - Act as the traffic manager for large
   downloads (extensions, language servers, dependencies) with resilient,
   resume-capable transfers through `Downloader/` and `Resilience/`.
4. **Isolate Security-Critical Operations** - Manage cryptographic signing,
   secure credential storage, and authentication token lifecycle via `Security/`
   and `Authentication/`, keeping sensitive logic isolated from the main
   application process.

---

## Key Features&#x2001;🪁

**`gRPC` Native Communication** - All inter-process communication with
**Mountain** ⛰️ travels over a **Vine** 🌿 (`tonic`-based `gRPC`) channel on
`[::1]:50053`, providing strongly-typed `protobuf` contracts, bi-directional
streaming for progress events, and a well-defined API surface generated from
`Air.proto`.

**Self-Contained Daemon Lifecycle** - Runs as an independent process with
singleton enforcement via PID locking in `Daemon/`, graceful shutdown on
`SIGTERM` managed by `Binary/Shutdown/WaitForShutdownSignal.rs`, and
platform-native daemonization on `macOS`, `Linux`, and `Windows`. Survives
window closures and persists across editor sessions.

**Resilient Update Engine** - Full ownership of the update lifecycle: version
checking against multiple channels (stable, beta, nightly) in `Updates/`,
concurrent chunked downloads with resume capability in `Downloader/`,
cryptographic checksum verification via `Security/ChecksumVerifier.rs`, staged
installation, and automatic rollback on failure via `Updates/RollbackState.rs`.

**Isolated Security Boundaries** - Cryptographic signing with `ring`, AEAD
encrypted credential storage with `zeroize` in `Security/SecureStorage.rs`,
token lifecycle management in `Authentication/`, rate limiting via token bucket
in `Security/TokenBucket.rs`, and a comprehensive security audit subsystem in
`Security/SecurityAuditor.rs` - all isolated from the main application process.

**Real-Time File Indexing** - Persistent file index with `Rust` and `TypeScript`
symbol extraction in `Indexing/Language/`, recursive directory scanning via
`Indexing/Scan/`, `notify`-based file system watching for live updates in
`Indexing/Watch/`, and a fast query engine with fuzzy search across the entire
workspace in `Indexing/Store/`.

**Observability by Default** - Structured `JSON` logging with trace-ID
propagation in `Logging/`, `OpenTelemetry`-compatible distributed tracing with
configurable sampling in `Tracing/`, and `Prometheus`-compatible metrics for
latency, success rates, and resource utilization across every service in
`Metrics/`.

**Resilience Everywhere** - All network operations are wrapped in retry-with-
exponential-backoff via `Resilience/Retry.rs`, circuit breakers with half-open
probing in `Resilience/CircuitBreaker.rs`, bulkhead executors for service
isolation in `Resilience/BulkheadExecutor.rs`, and configurable timeouts via
`Resilience/Timeout.rs`.

---

## Core Architecture Principles&#x2001;🏗️

| Principle                      | Description                                                                                                                          | Key Components                                            |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| **Sidecar Isolation**          | Run as a standalone daemon process, surviving independently of the main window lifecycle for persistent background operations.       | `Daemon/`, `Binary/`, PID locking                         |
| **`gRPC` IPC Boundary**        | Use **Vine** 🌿 (`tonic`-based `gRPC`) for all communication with **Mountain** ⛰️, ensuring a high-performance and well-defined API. | `Vine/`, `Air.proto`, generated `prost` bindings          |
| **Service Modularity**         | Each capability (updates, downloads, auth, indexing) lives in its own module with independent startup and health monitoring.         | `Updates/`, `Downloader/`, `Authentication/`, `Indexing/` |
| **Resilience by Default**      | Wrap all network operations in retry-with-backoff, circuit breakers, bulkheads, and timeouts via the shared `Resilience/` library.   | `Resilience/`, `HealthCheck/`                             |
| **Secure Credential Handling** | Never expose raw secrets; store credentials with AEAD encryption (`ring`), enforce key rotation, and audit all access.               | `Security/`, `Authentication/`, `zeroize`                 |

---

## System Architecture&#x2001;

```mermaid
graph LR
    classDef mountain fill:#f0d0ff,stroke:#9b59b6,stroke-width:2px,color:#2c0050;
    classDef air      fill:#e0f4ff,stroke:#2471a3,stroke-width:2px,color:#001040;
    classDef external fill:#ebebeb,stroke:#888,stroke-width:1px,stroke-dasharray:5 5,color:#333;
    classDef infra    fill:#fff3c0,stroke:#f39c12,stroke-width:1px,stroke-dasharray:5 5,color:#5a3e00;

    subgraph MOUNTAIN["Mountain ⛰️ - Main Application"]
        MountainIPC["Mountain gRPC client delegates heavy tasks"]:::mountain
    end

    subgraph AIR["Air 🪁 - Persistent Background Daemon (::1:50053)"]
        direction TB
        subgraph COMM["Vine/ - gRPC Transport"]
            VineServer["Vine/Server/ - gRPC server (Generated/ prost bindings)"]:::air
            MountainClient["Mountain gRPC client (Air → Mountain callbacks)"]:::air
        end
        subgraph CORE["Core Services"]
            Updates["Updates/ - version check, download, verify, staged install, rollback"]:::air
            Downloader["Downloader/ - parallel chunks, rate-limit, resume, retry"]:::air
            Auth["Authentication/ - token mgmt, AEAD encrypt, key rotation"]:::air
            Indexing["Indexing/ - file index, symbol extract, FS watch, search"]:::air
        end
        subgraph INFRA["Infrastructure"]
            Health["HealthCheck/ - Alive/Responsive/Functional, auto-recovery"]:::infra
            Resilience["Resilience/ - retry backoff, circuit breaker, bulkhead"]:::infra
            Metrics["Metrics/ - Prometheus-compatible, latency, success rate"]:::infra
            Security["Security/ - AES-GCM, checksum, audit"]:::infra
            Daemon["Daemon/ - PID lock, singleton enforce"]:::air
        end

        VineServer --> Updates
        VineServer --> Downloader
        VineServer --> Auth
        VineServer --> Indexing
        Updates --> Resilience
        Downloader --> Resilience
    end

    subgraph EXTERNAL["External ☁️"]
        UpdateSrv["Update servers / extension registry"]:::external
    end

    MountainIPC -- gRPC :50053 --> VineServer
    MountainClient -- progress events --> MountainIPC
    Updates -- fetches --> UpdateSrv
    Downloader -- downloads --> UpdateSrv
```

**Connection paths:**

| Path                                          | Protocol                             | Use Case                                    |
| --------------------------------------------- | ------------------------------------ | ------------------------------------------- |
| **Mountain** ⛰️ → **Air** via `gRPC`          | `protobuf` over `gRPC` on port 50053 | Delegate updates, downloads, indexing, auth |
| **Air** → **Mountain** ⛰️ via `gRPC` callback | `protobuf` over `gRPC`               | Progress events, health status, metrics     |
| **Air** → External via `HTTP`                 | `HTTPS` with `Mist` 🌫️ DNS isolation | Update servers, extension registries        |

---

## Key Components

| Component          | Path                       | Description                                                                                                   |
| ------------------ | -------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Binary Entry Point | `Source/Binary.rs`         | Binary entry point for the Air daemon                                                                         |
| Library Root       | `Source/Library.rs`        | Module declarations and crate-level exports                                                                   |
| Daemon Lifecycle   | `Source/Binary/`           | Daemon process lifecycle (startup, shutdown, monitoring)                                                      |
| Singleton Enforcer | `Source/Daemon/`           | Singleton enforcement, PID locking, platform-native integration                                               |
| Initialization     | `Source/Initialize/`       | Configuration, port binding, `gRPC` server construction, per-service startup                                  |
| CLI Interface      | `Source/CLI/`              | Command-line interface for daemon interaction and diagnostics                                                 |
| `gRPC` Client      | `Source/Client/`           | Typed `gRPC` client (`AirClient`) and service provider (`AirServiceProvider`) for **Mountain** ⛰️ interaction |
| `gRPC` Protocol    | `Source/Vine/`             | `gRPC` protocol implementation (generated `prost` bindings, server, errors)                                   |
| Application State  | `Source/ApplicationState/` | Central coordination (connections, service states, telemetry, resources)                                      |
| Configuration      | `Source/Configuration/`    | `TOML` config loading with schema validation, env overrides, hot reload                                       |
| Updates            | `Source/Updates/`          | Version checking, download, verification, staged install, rollback                                            |
| Downloader         | `Source/Downloader/`       | Parallel downloads, chunk transfers, rate limiting, resume capability                                         |
| Authentication     | `Source/Authentication/`   | Token management, credential storage, AEAD encryption, key rotation                                           |
| Indexing Engine    | `Source/Indexing/`         | File index, symbol extraction, scanning, persistent storage, FS watch                                         |
| Health Check       | `Source/HealthCheck/`      | Multi-level health monitoring (alive, responsive, functional) with auto-recovery                              |
| Logging            | `Source/Logging/`          | Structured `JSON` logging with trace-ID propagation, rotation, sensitive data filtering                       |
| Metrics            | `Source/Metrics/`          | `Prometheus`-compatible metrics (latency, success rate, resource usage)                                       |
| Resilience         | `Source/Resilience/`       | Retry with exponential backoff, circuit breaker, bulkhead, timeout management                                 |
| Security           | `Source/Security/`         | Checksum verification, AES-GCM credential storage, rate limiting, audit subsystem                             |
| Tracing            | `Source/Tracing/`          | Distributed tracing with sampling, span events, context propagation                                           |
| HTTP Client        | `Source/HTTP/`             | Secure HTTP client with custom DNS via `Mist` 🌫️, `TLS`, timeout management                                   |
| Mountain Bridge    | `Source/Mountain/`         | Client for **Mountain** ⛰️ callbacks with `TLS` configuration                                                 |
| Plugin System      | `Source/Plugins/`          | Plugin discovery, loading, sandboxing, event bus, and capability management                                   |

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
│   │   ├── mod.rs
│   │   ├── ApplicationState.rs   # Central application state
│   │   ├── ConnectionHealthReport.rs
│   │   ├── ConnectionInfo.rs
│   │   ├── ConnectionType.rs
│   │   ├── PerformanceMetrics.rs
│   │   ├── RequestState.rs
│   │   ├── RequestStatus.rs
│   │   ├── ResourceUsage.rs
│   │   └── ServiceStatus.rs
│   ├── Authentication/
│   │   ├── mod.rs
│   │   ├── AuthenticationService.rs
│   │   ├── AuthSession.rs
│   │   ├── CredentialsStore.rs
│   │   └── CryptoKeys.rs
│   ├── Binary/
│   │   ├── mod.rs
│   │   ├── Binary.rs             # Binary initialization
│   │   ├── Monitor/
│   │   │   └── StartMonitoring.rs
│   │   └── Shutdown/
│   │       └── WaitForShutdownSignal.rs
│   ├── CLI/
│   │   ├── mod.rs
│   │   ├── CliHandler.rs
│   │   ├── CliParser.rs
│   │   ├── CommandTypes.rs
│   │   ├── DaemonClient.rs
│   │   ├── OutputFormat.rs
│   │   ├── OutputFormatter.rs
│   │   ├── ResponseTypes.rs
│   │   └── Tests.rs
│   ├── Client/
│   │   ├── mod.rs
│   │   ├── AirClient/            # Typed gRPC client methods
│   │   │   ├── mod.rs
│   │   │   ├── AirMetrics.rs
│   │   │   ├── AirStatus.rs
│   │   │   ├── ApplyUpdate.rs
│   │   │   ├── Authenticate.rs
│   │   │   ├── CheckForUpdates.rs
│   │   │   ├── DownloadFile.rs
│   │   │   ├── DownloadStream.rs
│   │   │   ├── DownloadStreamChunk.rs
│   │   │   ├── DownloadStreamRpc.rs
│   │   │   ├── DownloadUpdate.rs
│   │   │   ├── ExtendedFileInfo.rs
│   │   │   ├── FileInfo.rs
│   │   │   ├── FileResult.rs
│   │   │   ├── GetConfiguration.rs
│   │   │   ├── GetFileInfo.rs
│   │   │   ├── GetMetrics.rs
│   │   │   ├── GetResourceUsage.rs
│   │   │   ├── GetStatus.rs
│   │   │   ├── HealthCheck.rs
│   │   │   ├── IndexFiles.rs
│   │   │   ├── IndexInfo.rs
│   │   │   ├── ResourceUsage.rs
│   │   │   ├── SearchFiles.rs
│   │   │   ├── SetResourceLimits.rs
│   │   │   ├── UpdateConfiguration.rs
│   │   │   └── UpdateInfo.rs
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
│   │   ├── AirConfiguration.rs
│   │   ├── ConfigurationManager.rs
│   │   ├── HotReload.rs
│   │   ├── Schema.rs
│   │   └── Tests.rs
│   ├── Daemon/
│   │   ├── mod.rs
│   │   ├── DaemonManager.rs
│   │   ├── DaemonStatus.rs
│   │   ├── ExitCode.rs
│   │   ├── Platform.rs
│   │   └── PlatformInfo.rs
│   ├── Downloader/
│   │   ├── mod.rs
│   │   ├── DownloadManager.rs
│   │   ├── RateLimit.rs
│   │   └── Types.rs
│   ├── HealthCheck/
│   │   ├── mod.rs
│   │   ├── DegradationLevel.rs
│   │   ├── HealthCheckConfig.rs
│   │   ├── HealthCheckLevel.rs
│   │   ├── HealthCheckManager.rs
│   │   ├── HealthCheckRecord.rs
│   │   ├── HealthCheckResponse.rs
│   │   ├── HealthStatistics.rs
│   │   ├── HealthStatus.rs
│   │   ├── PerformanceIndicators.rs
│   │   ├── RecoveryAction.rs
│   │   ├── RecoveryActionType.rs
│   │   ├── RecoveryTrigger.rs
│   │   ├── ResourceWarning.rs
│   │   ├── ResourceWarningType.rs
│   │   ├── ServiceHealth.rs
│   │   └── WarningSeverity.rs
│   ├── HTTP/
│   │   ├── mod.rs
│   │   └── Client.rs
│   ├── Indexing/
│   │   ├── mod.rs
│   │   ├── FileIndexer.rs
│   │   ├── IndexResult.rs
│   │   ├── Background/
│   │   │   ├── mod.rs
│   │   │   └── StartWatcher.rs
│   │   ├── Language/
│   │   │   ├── mod.rs
│   │   │   ├── ParseRust.rs
│   │   │   └── ParseTypeScript.rs
│   │   ├── Process/
│   │   │   ├── mod.rs
│   │   │   ├── ExtractSymbols.rs
│   │   │   └── ProcessContent.rs
│   │   ├── Scan/
│   │   │   ├── mod.rs
│   │   │   ├── ScanDirectory.rs
│   │   │   └── ScanFile.rs
│   │   ├── State/
│   │   │   ├── mod.rs
│   │   │   ├── CreateState.rs
│   │   │   └── UpdateState.rs
│   │   ├── Store/
│   │   │   ├── mod.rs
│   │   │   ├── QueryIndex.rs
│   │   │   ├── StoreEntry.rs
│   │   │   └── UpdateIndex.rs
│   │   └── Watch/
│   │       ├── mod.rs
│   │       └── WatchFile.rs
│   ├── Initialize/
│   │   ├── mod.rs
│   │   ├── Build/
│   │   │   ├── mod.rs
│   │   │   └── BuildServer.rs
│   │   ├── Command/
│   │   │   ├── mod.rs
│   │   │   ├── Connect/
│   │   │   │   ├── mod.rs
│   │   │   │   └── ConnectDaemon.rs
│   │   │   ├── HandleCommand.rs
│   │   │   ├── ParseArguments.rs
│   │   │   └── ValidateCommand.rs
│   │   ├── Configure/
│   │   │   ├── mod.rs
│   │   │   ├── Log/
│   │   │   │   └── ConfigureLog.rs
│   │   │   └── Port/
│   │   │       └── SelectPort.rs
│   │   └── Service/
│   │       ├── mod.rs
│   │       ├── Auth/
│   │       │   └── StartAuth.rs
│   │       ├── Download/
│   │       │   └── StartDownload.rs
│   │       ├── Echo/
│   │       │   └── StartEcho.rs
│   │       ├── Health/
│   │       │   └── StartHealthCheck.rs
│   │       ├── Index/
│   │       │   └── StartIndex.rs
│   │       ├── State/
│   │       │   └── CreateState.rs
│   │       ├── Update/
│   │       │   └── StartUpdate.rs
│   │       └── Vine/
│   │           └── StartService.rs
│   ├── Logging/
│   │   ├── mod.rs
│   │   ├── ContextLogger.rs
│   │   ├── LogContext.rs
│   │   ├── LogManager.rs
│   │   ├── LogRotationConfig.rs
│   │   ├── SensitiveDataConfig.rs
│   │   ├── SensitiveDataFilter.rs
│   │   └── StructuredLogEntry.rs
│   ├── Metrics/
│   │   ├── mod.rs
│   │   ├── AggregationValidator.rs
│   │   ├── GetMetrics.rs
│   │   ├── MetricGuard.rs
│   │   ├── MetricsCollector.rs
│   │   ├── MetricsData.rs
│   │   └── MinMaxUpdate.rs
│   ├── Mountain/
│   │   ├── mod.rs
│   │   ├── Constants.rs
│   │   ├── MountainClient.rs
│   │   ├── MountainClientConfig.rs
│   │   └── TlsConfig.rs
│   ├── Plugins/
│   │   ├── mod.rs
│   │   ├── ApiVersion.rs
│   │   ├── EventBus.rs
│   │   ├── Plugin.rs
│   │   ├── PluginCapability.rs
│   │   ├── PluginDependency.rs
│   │   ├── PluginDiscoveryResult.rs
│   │   ├── PluginHooks.rs
│   │   ├── PluginInfo.rs
│   │   ├── PluginLoader.rs
│   │   ├── PluginManager.rs
│   │   ├── PluginManifest.rs
│   │   ├── PluginMessage.rs
│   │   ├── PluginMetadata.rs
│   │   ├── PluginPermission.rs
│   │   ├── PluginRegistry.rs
│   │   ├── PluginSandboxConfig.rs
│   │   ├── PluginSandboxManager.rs
│   │   ├── PluginState.rs
│   │   ├── PluginValidationResult.rs
│   │   └── Test.rs
│   ├── Resilience/
│   │   ├── mod.rs
│   │   ├── BulkheadConfig.rs
│   │   ├── BulkheadExecutor.rs
│   │   ├── BulkheadStatistics.rs
│   │   ├── CircuitBreaker.rs
│   │   ├── CircuitBreakerConfig.rs
│   │   ├── CircuitEvent.rs
│   │   ├── CircuitState.rs
│   │   ├── CircuitStatistics.rs
│   │   ├── ResilienceOrchestrator.rs
│   │   ├── ResilienceTests.rs
│   │   ├── Retry.rs
│   │   └── Timeout.rs
│   ├── Security/
│   │   ├── mod.rs
│   │   ├── ChecksumVerifier.rs
│   │   ├── RateLimitConfig.rs
│   │   ├── RateLimiter.rs
│   │   ├── RateLimitStatus.rs
│   │   ├── SecureBytes.rs
│   │   ├── SecureStorage.rs
│   │   ├── SecurityAuditor.rs
│   │   ├── SecurityEvent.rs
│   │   ├── SecurityEventType.rs
│   │   ├── SecuritySeverity.rs
│   │   ├── SecurityTests.rs
│   │   └── TokenBucket.rs
│   ├── Tracing/
│   │   ├── mod.rs
│   │   ├── PropagationContext.rs
│   │   ├── SamplingConfig.rs
│   │   ├── SpanEvent.rs
│   │   ├── SpanStatus.rs
│   │   ├── TraceGenerator.rs
│   │   ├── TraceMetadata.rs
│   │   ├── TraceSpan.rs
│   │   └── TraceStatistics.rs
│   ├── Updates/
│   │   ├── mod.rs
│   │   ├── ChecksumUtil.rs
│   │   ├── DownloadSession.rs
│   │   ├── InstallationStatus.rs
│   │   ├── PackageFormat.rs
│   │   ├── PlatformConfig.rs
│   │   ├── PlatformDetect.rs
│   │   ├── PlatformMetadata.rs
│   │   ├── RollbackHistory.rs
│   │   ├── RollbackState.rs
│   │   ├── Types.rs
│   │   ├── UpdateChannel.rs
│   │   ├── UpdateInfo.rs
│   │   ├── UpdateManager.rs
│   │   ├── UpdateStatus.rs
│   │   ├── UpdateTelemetry.rs
│   │   └── VersionCompare.rs
│   └── Vine/
│       ├── mod.rs
│       ├── Error.rs
│       ├── Generated/
│       │   ├── mod.rs
│       │   └── air.rs
│       └── Server/
│           ├── mod.rs
│           └── AirVinegRPCService.rs
└── Documentation/
    ├── GitHub/
    │   ├── Architecture.md
    │   └── DeepDive.md
    └── Rust/
        └── doc/                  # Cargo doc output
```

---

## In the Land Project

**Air** 🪁 is the persistent background daemon for the Land ecosystem. It
communicates with **Mountain** ⛰️ via **Vine** 🌿 (`gRPC`) on port `[::1]:50053`
and uses **Mist** 🌫️ for DNS isolation on its HTTP client.

| Role                | Details                                                                                        |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| **Daemon Process**  | Persistent executable that runs independently of the main window, even after the window closes |
| **Server Host**     | Hosts a local `gRPC` server on `[::1]:50053` to accept commands from **Mountain** ⛰️           |
| **Update Delegate** | Sole authority for modifying installation files of the parent application                      |
| **Signer**          | Handles cryptographic signing of artifacts and secure token storage for user login             |
| **Traffic Manager** | Proxy/downloader that keeps large network operations off the main renderer process             |
| **File Indexer**    | Maintains a persistent file index with symbol extraction and fast search across the workspace  |
| **Health Monitor**  | Periodically checks all service health with automatic recovery and degradation tracking        |

### Port Allocation

| Process       | Port    | Protocol                           | Purpose                                        |
| ------------- | ------- | ---------------------------------- | ---------------------------------------------- |
| **Air** 🪁    | `50053` | **Vine** 🌿 / `Air.proto` (`gRPC`) | Daemon services - updates, downloads, indexing |
| **Cocoon** 🦋 | `50052` | `Vine.proto` (`gRPC`)              | VS Code extension hosting                      |

**Air** is part of the networking/IPC connectivity stack alongside **Mist** 🌫️
(DNS isolation) and **Vine** 🌿 (`gRPC` protocol layer).

Typical usage flow:

1. **Spawn:** **Mountain** ⛰️ detects if **Air** is running. If not, it spawns
   the binary.
2. **Connect:** **Mountain** ⛰️ establishes a **Vine** 🌿 (`gRPC`) connection to
   **Air**'s local port `[::1]:50053`.
3. **Delegate:** When a user requests an update or large download, **Mountain**
   ⛰️ sends a command to **Air** and immediately returns control to the user.
4. **Monitor:** **Air** emits progress events back to **Mountain** ⛰️ to update
   the UI status bars.

---

## Getting Started&#x2001;🚀

### Prerequisites

- **`Rust`** 1.75 or later
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

| Feature          | Description                                                   |
| ---------------- | ------------------------------------------------------------- |
| `default`        | Enables `full-services` and `mtls`                            |
| `full-services`  | Enables `authentication`, `updates`, `downloader`, `indexing` |
| `authentication` | Token management and credential storage                       |
| `updates`        | Update lifecycle management                                   |
| `downloader`     | Parallel chunked downloads with resume                        |
| `indexing`       | File indexing with symbol extraction                          |
| `mtls`           | Mutual `TLS` for `gRPC` connections                           |
| `appimage`       | `AppImage` package format support                             |
| `deb`            | `Debian` package format support                               |
| `rpm`            | `RPM` package format support                                  |

### Build with Features

```bash
# Default features (full-services + `mTLS`)
cargo build --release

# Minimal daemon (no update/auth/indexing)
cargo build --release --no-default-features

# All features
cargo build --release --all-features
```

### Key Dependencies

| Crate / Package          | Purpose                                                        |
| ------------------------ | -------------------------------------------------------------- |
| `tonic` / `prost`        | `gRPC` server and Protocol Buffer code generation              |
| **Vine** 🌿              | Local path dependency - generated `Air.proto` `gRPC` contracts |
| `Common`                 | Local path dependency - shared types and abstractions          |
| **Mist** 🌫️              | Local path dependency - DNS isolation for HTTP client          |
| `reqwest` / `rustls`     | `HTTPS` downloads with `TLS` certificate verification          |
| `tokio`                  | Async runtime for concurrent I/O and task scheduling           |
| `notify` / `ignore`      | File system event watching for real-time index updates         |
| `ring` / `zeroize`       | Cryptographic signing and secure credential storage            |
| `tracing`                | Structured `JSON` logging with span propagation                |
| `config` / `toml`        | Configuration file loading with hot-reload support             |
| `sysinfo` / `systemstat` | System resource monitoring and health checks                   |
| `walkdir` / `ignore`     | Recursive directory traversal for file indexing                |

---

## Security&#x2001;🔒

**Air** enforces security at multiple layers, isolating sensitive operations
from the main application process:

| Layer                     | Mechanism                                                                         |
| ------------------------- | --------------------------------------------------------------------------------- |
| **Process Isolation**     | Separate daemon process - cryptographic and auth logic never runs in the renderer |
| **Network**               | `mTLS` for `gRPC` connections, `Mist` 🌫️ DNS isolation for outbound HTTP          |
| **Credentials**           | AEAD encryption via `ring`, `zeroize`-protected memory, key rotation              |
| **Rate Limiting**         | Token bucket algorithm per-endpoint rate limiting                                 |
| **Checksum Verification** | All downloaded artifacts verified via `SHA-256` / `MD5` before installation       |
| **Audit Logging**         | Security audit subsystem with severity-classified events                          |
| **Singleton Enforcement** | PID locking prevents duplicate daemon instances                                   |

---

## Compatibility

**Air** is designed to be compatible with:

| Target          | Integration                                                                          |
| --------------- | ------------------------------------------------------------------------------------ |
| **Mountain** ⛰️ | Communicates via `gRPC` on port 50053 - delegates updates, downloads, indexing, auth |
| **Vine** 🌿     | Uses `Air.proto` `gRPC` contracts for all inter-process communication                |
| **Mist** 🌫️     | Uses DNS isolation for all outbound HTTP requests                                    |
| **Cocoon** 🦋   | Shares port allocation awareness - Cocoon occupies port 50052, Air occupies 50053    |
| **Echo** 📣     | StartEcho service initializes Echo task scheduling within the daemon process         |

---

## API Reference

- **[Rust API Documentation](https://rust.documentation.air.editor.land/)**&#x2001;📖
- [Deep Dive](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/DeepDive.md)
    - Detailed startup sequence, `gRPC` routing, and data flow

---

## Related Documentation

- [Architecture Overview](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/Architecture.md)
    - Internal module structure
- [Deep Dive](https://github.com/CodeEditorLand/Air/tree/Current/Documentation/GitHub/DeepDive.md)
    - In-depth technical details
- [Land Documentation](../../Documentation/GitHub/README.md) - Complete
  documentation index
- **Mountain** ⛰️ - Main application process -
  [GitHub](https://github.com/CodeEditorLand/Mountain)
- **Mist** 🌫️ - DNS isolation for the private network -
  [GitHub](https://github.com/CodeEditorLand/Mist)
- **Vine** 🌿 - `gRPC` protocol layer -
  [GitHub](https://github.com/CodeEditorLand/Vine)
- **Cocoon** 🦋 - `Node.js`/`Effect-TS` extension host -
  [GitHub](https://github.com/CodeEditorLand/Cocoon)
- **Grove** 🌳 - `Rust`/`WASM` extension host -
  [GitHub](https://github.com/CodeEditorLand/Grove)
- **Echo** 📣 - Task scheduler -
  [GitHub](https://github.com/CodeEditorLand/Echo)
- **Common** - Shared types and abstractions -
  [GitHub](https://github.com/CodeEditorLand/Common)

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
