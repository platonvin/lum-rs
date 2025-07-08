use crate::spirv_builder::Capability;
use crate::spirv_builder::MetadataPrintout;
use crate::spirv_builder::SpirvMetadata;
use cargo_gpu::*;
use std::path::PathBuf;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_crate = PathBuf::from("../shader_sources");

    // install the toolchain and build the `rustc_codegen_spirv` codegen backend with it
    let backend = cargo_gpu::Install::from_shader_crate(shader_crate.clone()).run()?;

    // build the shader crate
    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.2");
    builder.print_metadata = MetadataPrintout::DependencyOnly;
    builder.spirv_metadata = SpirvMetadata::Full;
    builder.capabilities.push(Capability::InputAttachment);
    builder.capabilities.push(Capability::Int16);
    builder.capabilities.push(Capability::StorageImageExtendedFormats);
    builder.capabilities.push(Capability::ImageQuery);
    builder.capabilities.push(Capability::Int8);
    builder.capabilities.push(Capability::DerivativeControl);
    builder.capabilities.push(Capability::GroupNonUniform);
    builder.capabilities.push(Capability::GroupNonUniformArithmetic);
    builder.capabilities.push(Capability::StorageImageReadWithoutFormat);
    let spv_result = builder.build()?;
    let path_to_spv = spv_result.module.unwrap_single();

    println!("cargo::rustc-env=SHADER_PATH={}", path_to_spv.display());

    Ok(())
}

/*
what was frustraring:
no example for library
cargo features 2024 wtf?
implicit locations

WHY the fuck did i need to put these
#[unsafe(no_mangle)]
#[inline(never)]
to get my passtrough functino working? (i mean compiler just removed the thing but please stop it)
need to list all attributes and their example usage on main page
colored output in terminale

error: [VUID-StandaloneSpirv-OpTypeImage-04657] Sampled must be 1 or 2 in the Vulkan environment


what the fuck with repr? its not repr C???????
 */
