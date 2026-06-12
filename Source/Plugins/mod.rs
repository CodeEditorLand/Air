//! # Plugin Architecture
//!
//! ## Responsibilities
//!
//! This module provides a comprehensive plugin system for the Air daemon,
//! enabling extensibility through dynamically loaded plugins that can enhance
//! daemon functionality. The plugin system is responsible for:
//!
//! - **Plugin Discovery**: Automatically discovering available plugins from
//!   configured directories
//! - **Plugin Loading**: Dynamically loading plugins into the daemon runtime
//! - **Plugin Validation**: Validating plugin metadata, dependencies, and
//!   compatibility
//! - **Sandboxing**: Isolating plugins to prevent crashes and security issues
//! - **Lifecycle Management**: Managing plugin states (load, start, stop,
//!   unload) with proper hooks
//! - **API Registration**: Extending the daemon API through plugin-provided
//!   commands and handlers
//! - **Inter-Plugin Communication Enabling**: plugins to communicate with each
//!   other via message passing
//! - **Permission Management**: Enforcing fine-grained permissions and
//!   capabilities for plugins
//! - **Version Compatibility**: Ensuring plugins are compatible with the daemon
//!   version
//! - **Dependency Resolution**: Resolving and validating plugin dependencies
//!
//! ## VSCode Extension Architecture Patterns
//!
//! This implementation draws inspiration from VSCode's extension architecture:
//! - Reference: vs/platform/extensions/common/ extensionHostStarter.ts
//! - Reference: vs/server/node/ extensionHostConnection.ts
//! - Reference: vs/platform/remote/common/ remoteAgentConnection.ts
//!
//! Patterns adopted from VSCode extensions:
//! - Separate extension host process for isolation and crash protection
//! - Activation events to trigger extension loading on-demand
//! - Contribution points for extending functionality
//! - Message-based communication between host and extensions
//! - State management and lifecycle hooks
//! - API versioning for backward compatibility
//! - Permission and capability descriptors
//!
//! ## Integration with Cocoon Extension Host
//!
//! The plugin system is designed to integrate with the Cocoon Extension Host
//! (similar to VSCode's extension host architecture). This provides:
//! - Isolated execution environments for plugins
//! - Crash recovery and resilience
//! - Resource management and limits
//! - Communication via IPC channels
//! - Hot reload capability without daemon restart
//!
//! ## FUTURE Enhancements
//!
//! - **Plugin Marketplace**: Implement a central plugin marketplace for
//! discovery and installation (similar to VSCode's extension marketplace)
//! - **Hot Reload Support**: Implement live reloading of plugins without daemon
//! restart
//! - **Advanced Sandboxing**: Add more sophisticated sandboxing with resource
//! quotas, network isolation, and filesystem access controls
//! - **Plugin Distribution**: Implement plugin packaging, signing, and
//! distribution mechanisms
//! - **Automatic Updates**: Add automatic plugin update checking and
//! installation
//! - **Telemetry Integration**: Add plugin usage telemetry and reporting
//! - **Plugin Profiles**: Support multiple plugin configurations for different
//! environments
//! - **Security Audit**: Implement comprehensive security audit and
//! vulnerability scanning for plugins
//! - **Performance Monitoring**: Add detailed performance monitoring and
//!   profiling for plugins
//! - **Plugin Debugging**: Provide debugging tools and interfaces for plugin
//!   developers
//!
//! ## Security and Isolation
//!
//! - Plugins run in isolated processes to prevent daemon crashes
//! - Fine-grained permission system controls plugin capabilities
//! - API version compatibility checks prevent breaking changes
//! - Resource limits prevent plugin exhaustion attacks
//! - Plugin authentication and signing to prevent malicious plugins
//! - Filesystem and network access restrictions

pub mod ApiVersion;

pub mod EventBus;

pub mod Plugin;

pub mod PluginCapability;

pub mod PluginDependency;

pub mod PluginDiscoveryResult;

pub mod PluginHooks;

pub mod PluginInfo;

pub mod PluginLoader;

pub mod PluginManager;

pub mod PluginManifest;

pub mod PluginMessage;

pub mod PluginMetadata;

pub mod PluginPermission;

pub mod PluginRegistry;

pub mod PluginSandboxConfig;

pub mod PluginSandboxManager;

pub mod PluginState;

pub mod PluginValidationResult;

pub mod Test;
