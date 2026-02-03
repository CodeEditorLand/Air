#![allow(non_snake_case)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("cargo:rerun-if-changed=Proto/Air.proto");

	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.out_dir("Source/Vine/Generated")
		.compile_well_known_types(true)
		.compile_protos(&["Proto/Air.proto"], &["Proto"])?;

	Ok(())
}
