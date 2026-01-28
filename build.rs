use std::io::Result;

fn main() -> Result<()> {
    // Generate gRPC code from proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("Source/Vine/Generated")
        .compile(&["Proto/air.proto"], &["Proto"])?;
    
    println!("cargo:rerun-if-changed=Proto/air.proto");
    
    Ok(())
}
