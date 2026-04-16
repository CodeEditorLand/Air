# Changelog

All notable changes to the Air element are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] — 2026 Q2

### Added

- Tag-filtered DevLog logging system
- Comprehensive developer documentation in README
- Linux package features

### Changed

- Applied consistent formatting across all source files
- Removed obsolete TODOs and fixed code formatting
- Normalized workflow arguments in GitHub Actions
- Updated Mist imports to PascalCase
- Renamed identifiers to PascalCase per project conventions

## [0.2.0] — 2026 Q1

### Added

- DNS-based security with local DNS resolver
- Mountain client with mTLS and downloader throttling
- File event handling and debouncing in Watch subsystem
- UpdateIndex module for incremental index synchronization
- Background file watcher and periodic indexing tasks
- StoreEntry module for index persistence and recovery
- QueryIndex module for searching and filtering indexed content
- UpdateState, CreateState modules for index data management
- ScanFile and ScanDirectory modules for file analysis
- ProcessContent module for file analysis pipeline
- ExtractSymbols module for unified parsing and utilities
- ParseTypeScript and ParseRust modules for symbol extraction
- StartService, StartUpdate, CreateState initialization modules
- StartIndex, StartHealthCheck, StartEcho, StartDownload, StartAuth modules
- SelectPort module for server binding configuration
- ConfigureLog module for structured logging setup
- ValidateCommand module for CLI input sanitization
- ParseArguments module for CLI and daemon argument processing
- HandleCommand module for CLI dispatch and daemon interaction
- ConnectDaemon module for daemon connectivity verification
- BuildServer module for gRPC service construction
- Signal handler for graceful daemon shutdown
- Background monitoring for resources and health checks
- Primary daemon entry point and orchestration logic
- Update channel selection support in configuration
- 50-level deep analysis audit template in Library
- Cryptographic utility dependencies

### Changed

- Implemented all 35 remaining Air TODOs
- Standardized TODO comments to FUTURE naming convention
- Standardized field naming to singular form in ApplicationState
- Cleaned up dead code and added lint attributes
- Improved documentation and cleaned up dead code
- Applied code formatting and refactored TLS configuration logic
- Committed debug and release binary artifacts via Git LFS
- Established project foundation with cargo config, git attributes, and
  licensing
- Enforced PascalCase naming conventions across entire codebase
- Finalized PascalCase migration and corrected external crate interactions
- Normalized naming conventions and consolidated gRPC service implementation
- Added generated gRPC bindings for AirService
- Enforced code formatting across all modules (Authentication, Binary,
  Configuration, Daemon, Downloader, HealthCheck, Indexing, Library, Logging,
  Metrics, Plugins, Resilience, Security, Tracing, Updates, Vine)
- Renamed library artifact to AirLibrary
- Overhauled update management with production-grade resilience and rollback
- Overhauled distributed tracing with W3C trace context and sensitive data
  protection
- Overhauled Resilience and Security modules with production-grade observability
- Overhauled plugin architecture with production-grade features
- Overhauled metrics module with thread-safe architecture
- Overhauled logging with rotation, sensitive data filtering, and validation
- Overhauled file indexing and search with VS Code compatibility
- Overhauled health check system with production-grade monitoring
- Overhauled download manager with production-grade resilience
- Overhauled daemon lifecycle management with production-grade features
- Overhauled configuration management with comprehensive validation
- Overhauled configuration hot-reload system
- Rewrote main entry point with comprehensive documentation
- Enhanced state management with validation, pooling, and health reporting
- Implemented comprehensive CLI command parsing with validation and daemon client

### Fixed

- Resolved various bugs and enhanced code quality
- Fixed module imports and streamlined download process
- Compilation errors and unnecessary mut warnings

## [0.1.0] — 2026 Q1

### Added

- Daemon lifecycle management and health monitoring
- gRPC service skeleton with authentication
- Core background services for downloader, indexing, and updates
- Generated gRPC client and server code

### Changed

- Standardized dependencies to use workspace versions
- Standardized path expansion and fixed type safety
- Refined code quality and removed redundancies

## [0.0.1] — 2025 Q3

### Added

- Initial project scaffolding
