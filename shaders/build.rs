use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

fn add_extension(path: &mut PathBuf, extension: impl AsRef<std::path::Path>) {
    match path.extension() {
        Some(ext) => {
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(extension.as_ref());
            path.set_extension(ext)
        }
        None => path.set_extension(extension.as_ref()),
    };
}

fn needs_recompile(source_path: &Path, output_path: &Path) -> bool {
    if !output_path.exists() {
        return true;
    }

    let source_modified = fs::metadata(source_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let output_modified = fs::metadata(output_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    source_modified > output_modified
}

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    let glslc_available = Command::new("glslc").arg("--version").output().is_ok();
    if !glslc_available {
        panic!("glslc not found. Shaders will not be compiled.");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("compiled_shaders");
    fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    for entry in fs::read_dir("shaders").expect("Failed to read shaders directory") {
        let entry = entry.expect("Failed to read shader entry");
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // for Vulkan
        let mut out_spv = out_dir.join(path.file_name().unwrap());
        add_extension(&mut out_spv, "spv");

        if !needs_recompile(&path, &out_spv) {
            // println!("Skipping up-to-date shader: {}", path.display());
            continue;
        }

        // current SPIR‑V pass:
        let status_spv = Command::new("glslc")
            .arg(&path)
            .arg("-DVULKANN") // define VULKAN
            // .arg("VULKAN")
            .arg("-o")
            .arg(&out_spv)
            .arg("--target-env=vulkan1.1")
            .arg("-g")
            .status()
            .expect("Failed to execute glslc");

        if !status_spv.success() {
            panic!("Failed to compile shader: {}", path.display());
        }
    }

    println!(
        "cargo:rustc-env=COMPILED_SHADERS_PATH={}/",
        out_dir.display()
    );
}
