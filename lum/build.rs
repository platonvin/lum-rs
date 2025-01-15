
fn main() {
    // You call it lazy
    // I call it precomputed for optimization
    let sources = [
        "cpp/my_cpp_code.cpp", 
        // "cpp/another_file.cpp"
    ];

    let mut build = cc::Build::new();
    build
        .compiler("clang++")
        .cpp(true) // not C
        .include("cpp/")
        .flag_if_supported("-std=c++17")
        // .flag_if_supported("-flto=thin") // TODO: does it work with Rust?
        .flag_if_supported("-O2")
        ;

        for source in sources {
        build.file(source);
    }

    // Compile into a static library
    build.compile("my_cpp_code");

    // I have no fucking idea how to actually get it working LOL
    // Add environment variables for Rust's linker
    // println!("cargo:rustc-link-arg=-fuse-ld=lld"); // Use LLVM's linker
    // println!("cargo:rustc-link-arg=-flto=thin");  // Enable ThinLTO for Rust
    println!("cargo:rerun-if-changed=cpp/my_cpp_code.cpp");
}