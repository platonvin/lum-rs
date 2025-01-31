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
        .flag_if_supported("-Os");

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
    // shaders();
    cpp();
}
