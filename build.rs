#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

use serde::Deserialize;

#[derive(Deserialize)]
struct Toml {
	package:Package,
}

#[derive(Deserialize)]
struct Package {
	version:String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("cargo:rerun-if-changed=Cargo.toml");

	println!(
		"cargo:rustc-env=CARGO_PKG_VERSION={}",
		toml::from_str::<Toml>(&std::fs::read_to_string("Cargo.toml").expect("Cannot Cargo.toml."))
			.expect("Cannot toml.")
			.package
			.version
	);

	println!("cargo:rerun-if-changed=Proto/Air.proto");

	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.out_dir("Source/Vine/Generated")
		.compile_well_known_types(true)
		.compile_protos(&["Proto/Air.proto"], &["Proto"])?;

	Ok(())
}
