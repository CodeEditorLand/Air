//! JSON Schema generation for Air configuration validation.
//!
//! `generate_schema()` returns a Draft-07 JSON Schema object describing every
//! field of `AirConfiguration`. The schema is used by `ConfigurationManager::
//! SchemaValidate` and can be exported for editor tooling or CI validation.

use serde_json::{Value as JsonValue, json};

/// Generate the JSON Schema (Draft-07) for `AirConfiguration`.
/// The returned object describes every sub-configuration section
/// (grpc, authentication, updates, downloader, indexing, logging,
/// performance) with their types, enums, ranges, and formats.
pub fn generate_schema() -> JsonValue {
	json!({
		"$schema": "http://json-schema.org/draft-07/schema#",
		"title": "Air Configuration Schema",
		"description": "Configuration schema for Air daemon",
		"type": "object",
		"required": ["SchemaVersion", "profile"],
		"properties": {
			"SchemaVersion": {
				"type": "string",
				"description": "Configuration schema version for migration tracking",
				"pattern": "^\\d+\\.\\d+\\.\\d+$"
			},
			"profile": {
				"type": "string",
				"description": "Profile name (dev, staging, prod, custom)",
				"enum": ["dev", "staging", "prod", "custom"]
			},
			"grpc": {
				"type": "object",
				"description": "gRPC server configuration",
				"properties": {
					"BindAddress": {
						"type": "string",
						"description": "gRPC server bind address",
						"format": "hostname-port"
					},
					"MaxConnections": {
						"type": "integer",
						"minimum": 10,
						"maximum": 10000
					},
					"RequestTimeoutSecs": {
						"type": "integer",
						"minimum": 1,
						"maximum": 3600
					}
				}
			},
			"authentication": {
				"type": "object",
				"description": "Authentication configuration",
				"properties": {
					"enabled": {"type": "boolean"},
					"CredentialsPath": {"type": "string"},
					"TokenExpirationHours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 8760
					},
					"MaxSessions": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1000
					}
				}
			},
			"updates": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"CheckIntervalHours": {
						"type": "integer",
						"minimum": 1,
						"maximum": 168
					},
					"UpdateServerUrl": {
						"type": "string",
						"pattern": "^https://"
					},
					"AutoDownload": {"type": "boolean"},
					"AutoInstall": {"type": "boolean"},
					"channel": {
						"type": "string",
						"enum": ["stable", "insiders", "preview"]
					}
				}
			},
			"downloader": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"MaxConcurrentDownloads": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					},
					"DownloadTimeoutSecs": {
						"type": "integer",
						"minimum": 10,
						"maximum": 3600
					},
					"MaxRetries": {
						"type": "integer",
						"minimum": 0,
						"maximum": 10
					},
					"CacheDirectory": {"type": "string"}
				}
			},
			"indexing": {
				"type": "object",
				"properties": {
					"enabled": {"type": "boolean"},
					"MaxFileSizeMb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1024
					},
					"FileTypes": {
						"type": "array",
						"items": {"type": "string"}
					},
					"UpdateIntervalMinutes": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1440
					},
					"IndexDirectory": {"type": "string"}
				}
			},
			"logging": {
				"type": "object",
				"properties": {
					"level": {
						"type": "string",
						"enum": ["trace", "debug", "info", "warn", "error"]
					},
					"FilePath": {"type": ["string", "null"]},
					"ConsoleEnabled": {"type": "boolean"},
					"MaxFileSizeMb": {
						"type": "integer",
						"minimum": 1,
						"maximum": 1000
					},
					"MaxFiles": {
						"type": "integer",
						"minimum": 1,
						"maximum": 50
					}
				}
			},
			"performance": {
				"type": "object",
				"properties": {
					"MemoryLimitMb": {
						"type": "integer",
						"minimum": 64,
						"maximum": 16384
					},
					"CPULimitPercent": {
						"type": "integer",
						"minimum": 10,
						"maximum": 100
					},
					"DiskLimitMb": {
						"type": "integer",
						"minimum": 100,
						"maximum": 102400
					},
					"BackgroundTaskIntervalSecs": {
						"type": "integer",
						"minimum": 1,
						"maximum": 3600
					}
				}
			}
		}
	})
}
