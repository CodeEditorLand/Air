use std::io::Result;

fn main() -> Result<()> {
    // Generate gRPC code from proto files
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("Source/Vine/Generated")
        .compile_well_known_types(true)
        .compile_protos(&["Proto/Air.proto"], &["Proto"])?;
    
    println!("cargo:rerun-if-changed=Proto/Air.proto");
    
    Ok(())
}
