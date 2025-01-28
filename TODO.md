inherit TODO from lum++
try_into<Type>().unwrap() -> as Type 
port C++ ogt mesher

profile query
stack Ring (FIFO_Ring / FIFRing) for known resources. How to have no-size stored reference in Rust? E.g. to "lumal.frame" for resource access

#repr C for types used in push constants
    vec3 / vec4

shader JIT

optimize update_radiance

vec3 -> vec4 SIMD asm check

generic subpasses (e.g. for ui)

vector (vec/mat) library that does not suck
    JUST FUCKING IMPLEMENT CASTS WHY EVERY SINGLE ONE OF THEM IS MISSING HALF KEY FEATURES

assume_assert

multiple VkFunCall's -> single VkFunCall

utilize copy queue

RUST-ANALYZER:
    suggest variable with attention to type
    

package profile:
Rust -> LLVM IR
C++ -> LLVM IR
Souper : LLVM IR -> LLVM IR 
Clang cross-lang LTO : LLVM IR -> asm
PGO

macro-driven pipelines:
    compile spv if not compiled
    auto format
    auto attributes
    push constants as array of structs


magicavoxel parser that does not suck
    dot_vox is so bad that i literally learned derivatives in less time i spend reading its code and i still have no idea what it does


resizing:
    impl
    sync problem try changing move() in the end_frame() or current() -> next() in some cases 
