use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let original_out_dir = PathBuf::from(env::var("OUT_DIR")?);
    tonic_build::configure()
        .file_descriptor_set_path(original_out_dir.join("service_descriptor.bin"))
        .compile_protos(&["proto/service.proto"], &["proto"])?;

    Ok(())
}
