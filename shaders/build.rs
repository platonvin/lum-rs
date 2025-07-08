use cargo_gpu::spirv_builder::{Capability, MetadataPrintout, SpirvMetadata};
use cargo_gpu::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf}; // Required for file writing

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_sources_dir = PathBuf::from("./sources"); // Directory containing your shader crates
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let generated_rs_path = out_dir.join("shaders.rs");

    let mut generated_enum_variants = Vec::new();
    let mut generated_match_arms = Vec::new();

    let mut shader_paths: Vec<(String, PathBuf)> = Vec::new();

    // Iterate over each shader crate in the shader_sources_dir
    for entry in fs::read_dir(&shader_sources_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check if it's a directory and contains a Cargo.toml (heuristic for a crate)
        if path.is_dir() && path.join("Cargo.toml").exists() {
            let crate_name = path.file_name().unwrap().to_string_lossy().to_string();
            println!("cargo:rerun-if-changed={}", path.display()); // Rerun build if shader crate changes

            // Install the toolchain and build the `rustc_codegen_spirv` backend
            let backend = Install::from_shader_crate(path.clone()).run()?;

            // Build the shader crate
            let mut builder = backend.to_spirv_builder(path, "spirv-unknown-vulkan1.2");
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

            // Store the path to the compiled SPIR-V
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

            // Add to our generated enum variants and match arms
            let enum_variant_name = convert_crate_name_to_enum_variant(&crate_name);
            generated_enum_variants.push(enum_variant_name.clone());
            generated_match_arms.push(format!(
                r#"            Self::{enum_variant} => include_bytes!(env!("{env_var_name}")),"#,
                enum_variant = enum_variant_name,
                env_var_name = env_var_name
            ));
        }
    }

    // Generate the Rust code for the enum and the get_shader function
    let mut file = fs::File::create(&generated_rs_path)?;
    writeln!(file, "/// An enum representing all compiled shaders.")?;
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
        "    /// Returns the SPIR-V bytecode for the given shader."
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
    println!("cargo:rerun-if-changed=build.rs"); // Rerun build if this script changes

    Ok(())
}

/// Converts a crate name (e.g., "grass-vert") into a valid enum variant (e.g., "GrassVert").
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
