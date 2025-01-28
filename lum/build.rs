use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

fn add_extension(path: &mut std::path::PathBuf, extension: impl AsRef<std::path::Path>) {
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
    // If the output file doesn't exist, we need to compile
    if !output_path.exists() {
        return true;
    }

    // Get the last modification time of the source file
    let source_modified = fs::metadata(source_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // Get the last modification time of the output file
    let output_modified = fs::metadata(output_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // Recompile if the source file is newer than the output file
    source_modified > output_modified
}

fn shaders() {
    // Tell Cargo to re-run this build script if any shader files change
    println!("cargo:rerun-if-changed=shaders");

    // Check if `glslc` is available
    let glslc_available = Command::new("glslc").arg("--version").output().is_ok();
    if !glslc_available {
        eprintln!("glslc not found. Shaders will not be compiled.");
        return;
    }

    // Create the output directory if it doesn't exist
    let out_dir = "shaders/compiled";
    fs::create_dir_all(out_dir).expect("Failed to create output directory");

    // Iterate over all shader files in the shaders directory
    for entry in fs::read_dir("shaders").expect("Failed to read shaders directory") {
        let entry = entry.expect("Failed to read shader entry");
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Set the output file path
        let mut out_path = Path::new(out_dir).join(path.file_name().unwrap()); // Keep the original file name (e.g., grass.frag)
        add_extension(&mut out_path, "spv"); // Append .spv to the original file name

        // Check if the shader needs to be recompiled
        if !needs_recompile(&path, &out_path) {
            println!("Skipping up-to-date shader: {}", path.display());
            continue;
        }

        // Compile the shader using glslc
        println!(
            "Compiling shader: {} -> {}",
            path.display(),
            out_path.display()
        );
        let status = Command::new("glslc")
            .arg(&path) // Input shader file
            .arg("-o")
            .arg(&out_path) // Output SPIR-V file
            .arg("-V")
            .arg("--target-env")
            .arg("vulkan1.1")
            .arg("-g")
            .status()
            .expect("Failed to execute glslc");

        if !status.success() {
            eprintln!("Failed to compile shader: {}", path.display());
            continue;
        }

        println!("Successfully compiled shader: {}", path.display());
    }
}

fn cpp() {
    // You call it lazy
    // I call it precomputed for optimization
    let sources = ["cpp/my_cpp_code.cpp", "cpp/ogt_vox.cpp"];

    let mut build = cc::Build::new();
    build
        .compiler("clang++")
        .cpp(true) // not C
        .include("cpp/")
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-flto=fat") // TODO: does it work with Rust?
        .flag_if_supported("-O3");

    for source in sources {
        build.file(source);
    }

    // Compile into a static library
    build.compile("cpp_code");

    // I have no fucking idea how to actually get it working LOL
    // its better to control it via .cargo/config.toml
    println!("cargo:rerun-if-changed=cpp/my_cpp_code.cpp");
    println!("cargo:rerun-if-changed=cpp/ogt_vox.cpp");
}

fn main() {
    shaders();
    cpp();
}
