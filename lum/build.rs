// fn main() {
//     cc::Build::new()
//         .compiler("clang++") // Specify the C++ compiler as clang++
//         .cpp(true) // Enables C++ compilation
//         .file("cpp/my_cpp_code.cpp") // Path to your C++ file
//         .include("cpp/") // Path to your C++ headers
//         .flag_if_supported("-std=c++17") // Add C++17 standard flag
//         .compile("my_cpp_code");
// }

fn main() {
    // You call it lazy
    // I call it precomputed for optimization
    let sources = [
        "cpp/my_cpp_code.cpp", 
        // "cpp/another_file.cpp"
    ];

    let mut build = cc::Build::new();
    build
        .compiler("clang++") // Specify the C++ compiler
        .cpp(true)           // Enables C++ compilation
        .include("cpp/")     // Path to your C++ headers
        .flag_if_supported("-std=c++17") // Use C++17
        // .flag_if_supported("-flto=thin") // Enable ThinLTO
        .flag_if_supported("-O2") // Optimize for release builds
        ;
    // Add all source files to the build
    for source in sources {
        build.file(source);
    }

    // Compile the sources into a static library
    build.compile("my_cpp_code");

    // I have no fucking idea how to actually get it working LOL
    // Add environment variables for Rust's linker
    // println!("cargo:rustc-link-arg=-fuse-ld=lld"); // Use LLVM's linker
    // println!("cargo:rustc-link-arg=-flto=thin");  // Enable ThinLTO for Rust
    println!("cargo:rerun-if-changed=cpp/my_cpp_code.cpp");
}