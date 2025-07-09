use cargo_gpu::spirv_builder::{Capability, MetadataPrintout, SpirvMetadata};
use cargo_gpu::*;
use std::{fs, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_sources_dir = PathBuf::from("./sources");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let generated_rs_path = out_dir.join("shaders.rs");

    let mut generated_enum_variants = Vec::new();
    let mut generated_match_arms = Vec::new();

    let mut shader_paths: Vec<(String, PathBuf)> = Vec::new();

    for entry in fs::read_dir(&shader_sources_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && path.join("Cargo.toml").exists() {
            let crate_name = path.file_name().unwrap().to_string_lossy().to_string();
            println!("cargo:rerun-if-changed={}", path.display());

            let backend = Install::from_shader_crate(path.clone()).run()?;

            let mut builder = backend.to_spirv_builder(path, "spirv-unknown-vulkan1.2"); // why is there no 1.3?
            builder.validator.scalar_block_layout = true;
            builder.print_metadata = MetadataPrintout::DependencyOnly;
            builder.spirv_metadata = SpirvMetadata::None;
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

            let env_var_name = format!(
                "SHADER_PATH_{}",
                crate_name.to_uppercase().replace('-', "_")
            );
            println!(
                "cargo::rustc-env={}={}",
                env_var_name,
                path_to_spv.display()
            );
            shader_paths.push((crate_name.clone(), path_to_spv.to_path_buf()));

            let enum_variant_name = convert_crate_name_to_enum_variant(&crate_name);
            generated_enum_variants.push(enum_variant_name.clone());
            generated_match_arms.push(format!(
                r#"            Self::{enum_variant} => include_bytes!(env!("{env_var_name}")),"#,
                enum_variant = enum_variant_name,
                env_var_name = env_var_name
            ));
        }
    }

    let mut file = fs::File::create(&generated_rs_path)?;
    writeln!(file, "/// An enum representing all compiled shaders")?;
    writeln!(file, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")?;
    writeln!(file, "pub enum Shader {{")?;
    for variant in &generated_enum_variants {
        writeln!(file, "    {},", variant)?;
    }
    writeln!(file, "}}")?;
    writeln!(file)?;

    writeln!(file, "impl Shader {{")?;
    writeln!(
        file,
        "    /// Returns the SPIR-V bytecode for the given shader"
    )?;
    writeln!(file, "    pub fn get_bytes(self) -> &'static [u8] {{")?;
    writeln!(file, "        match self {{")?;
    for arm in &generated_match_arms {
        writeln!(file, "{}", arm)?;
    }
    writeln!(file, "        }}")?;
    writeln!(file, "    }}")?;
    writeln!(file, "}}")?;

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn convert_crate_name_to_enum_variant(crate_name: &str) -> String {
    crate_name
        .split('_')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}
