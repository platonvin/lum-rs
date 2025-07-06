use crate::spirv_builder::MetadataPrintout;
use crate::spirv_builder::SpirvMetadata;
use cargo_gpu::spirv_builder::SpirvBuilder;
use cargo_gpu::*;
use std::path::PathBuf;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_crate = PathBuf::from("./sources");

    // install the toolchain and build the `rustc_codegen_spirv` codegen backend with it
    let backend = cargo_gpu::Install::from_shader_crate(shader_crate.clone()).run()?;

    // build the shader crate
    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.3");
    SpirvBuilder::
    // let mut builder_2 = backend. = ;
    builder
        .print_metadata = MetadataPrintout::DependencyOnly;
    builder.spirv_metadata = SpirvMetadata::Full;
    let spv_result = builder.build()?;
    let path_to_spv = spv_result.module.unwrap_single();

    println!("cargo::rustc-env=SHADER_PATH={}", path_to_spv.display());

    Ok(())
}

/*
what was frustraring:
no example for library
cargo features 2024 wtf?
 */
