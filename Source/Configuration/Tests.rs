use super::*;


#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn test_default_configuration() {
		let config = AirConfiguration::default();

		assert_eq!(config.SchemaVersion, "1.0.0");

		assert_eq!(config.Profile, "dev");

		assert!(config.Authentication.Enabled);

		assert!(config.Logging.ConsoleEnabled);
	}

	#[test]
	fn test_profile_defaults() {
		let DevConfig = ConfigurationManager::GetProfileDefaults("dev");

		assert_eq!(DevConfig.Profile, "dev");

		assert_eq!(DevConfig.Logging.Level, "debug");

		let ProdConfig = ConfigurationManager::GetProfileDefaults("prod");

		assert_eq!(ProdConfig.Profile, "prod");

		assert_eq!(ProdConfig.Logging.Level, "warn");

		assert!(!ProdConfig.Logging.ConsoleEnabled);
	}

	#[test]
	fn test_path_expansion() {
		let Home = dirs::home_dir().expect("Cannot determine home directory");

		let Expanded = ConfigurationManager::ExpandPath("~/test").unwrap();

		assert_eq!(Expanded, Home.join("test"));

		let Absolute = ConfigurationManager::ExpandPath("/tmp/test").unwrap();

		assert_eq!(Absolute, PathBuf::from("/tmp/test"));
	}

	#[test]
	fn test_address_validation() {
		assert!(ConfigurationManager::IsValidAddress("[::1]:50053"));

		assert!(ConfigurationManager::IsValidAddress("127.0.0.1:50053"));

		assert!(ConfigurationManager::IsValidAddress("localhost:50053"));

		assert!(!ConfigurationManager::IsValidAddress("invalid"));
	}

	#[test]
	fn test_url_validation() {
		assert!(ConfigurationManager::IsValidUrl("https://example.com"));

		assert!(ConfigurationManager::IsValidUrl("https://updates.editor.land"));

		assert!(!ConfigurationManager::IsValidUrl("not-a-url"));

		assert!(!ConfigurationManager::IsValidUrl("http://insecure.com"));
	}

	#[test]
	fn test_path_validation() {
		let manager = ConfigurationManager::New(None).unwrap();

		assert!(manager.ValidatePath("~/config").is_ok());

		assert!(manager.ValidatePath("/tmp/config").is_ok());

		assert!(manager.ValidatePath("../escaped").is_err());

		assert!(manager.ValidatePath("").is_err());
	}

	#[tokio::test]
	async fn test_export_import_json() {
		let config = AirConfiguration::default();

		let json_str = ConfigurationManager::ExportToJson(&config).unwrap();

		let imported = ConfigurationManager::ImportFromJson(&json_str).unwrap();

		assert_eq!(imported.SchemaVersion, config.SchemaVersion);

		assert_eq!(imported.Profile, config.Profile);

		assert_eq!(imported.gRPC.BindAddress, config.gRPC.BindAddress);
	}

	#[test]
	fn test_compute_hash() {
		let config = AirConfiguration::default();

		let hash1 = ConfigurationManager::ComputeHash(&config).unwrap();

		let hash2 = ConfigurationManager::ComputeHash(&config).unwrap();

		assert_eq!(hash1, hash2);

		let mut modified = config;

		modified.gRPC.BindAddress = "[::1]:50054".to_string();

		let hash3 = ConfigurationManager::ComputeHash(&modified).unwrap();

		assert_ne!(hash1, hash3);
	}

	#[test]
	fn test_generate_schema() {
		let schema = generate_schema();

		assert!(schema.is_object());

		assert!(schema.get("$schema").is_some());

		assert!(schema.get("properties").is_some());
	}
}
