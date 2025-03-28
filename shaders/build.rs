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
        eprintln!("glslc not found. Shaders will not be compiled.");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("compiled_shaders");
    fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    for entry in fs::read_dir("shaders").expect("Failed to read shaders directory") {
        let entry = entry.expect("Failed to read shader entry");
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let mut out_path = out_dir.join(path.file_name().unwrap());
        add_extension(&mut out_path, "spv");

        if !needs_recompile(&path, &out_path) {
            println!("Skipping up-to-date shader: {}", path.display());
            continue;
        }

        println!(
            "Compiling shader: {} -> {}",
            path.display(),
            out_path.display()
        );
        let status = Command::new("glslc")
            .arg(&path)
            .arg("-o")
            .arg(&out_path)
            .arg("--target-env=vulkan1.1")
            .arg("-g")
            .status()
            .expect("Failed to execute glslc");

        assert!(status.success());

        if !status.success() {
            eprintln!("Failed to compile shader: {}", path.display());
            continue;
        }

        println!("Successfully compiled shader: {}", path.display());
    }

    // inform Cargo about the compiled shaders directory so it can be included later
    println!(
        "cargo:rustc-env=COMPILED_SHADERS_PATH={}/",
        out_dir.display()
    );
}
