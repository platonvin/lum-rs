inherit TODO from lum++
try_into<Type>().unwrap() -> as Type 

profile query
stack Ring (FIFO_Ring / FIFRing) for known resources. How to have no-size stored reference in Rust? E.g. to "lumal.frame" for resource access

#repr C for types used in push constants
 - vec3 / vec4

shader JIT

optimize update_radiance

vec3 -> vec4 SIMD asm check

generic subpasses (e.g. for ui)

vector (vec/mat) library that does not suck
 - JUST FUCKING IMPLEMENT CASTS WHY EVERY SINGLE ONE OF THEM IS MISSING HALF KEY FEATURES
   - i8vec4 cant be casted into i16vec4???

assume_assert

multiple VkFunCall's -> single VkFunCall

utilize copy queue

RUST-ANALYZER:
 - suggest variable with attention to type

 scc actions build / test:
  scc complexity tracking
  binary sizes
  wgpu can be tested??
 - 

package profile:
Rust -> LLVM IR
C++ -> LLVM IR
Souper : LLVM IR -> LLVM IR 
Clang cross-lang LTO : LLVM IR -> asm
PGO

macro-driven pipelines:
 - compile spv if not compiled
 - auto format
 - auto attributes
 - push constants as array of structs


magicavoxel parser that does not suck
 - dot_vox is so bad that i literally learned derivatives in less time i spend reading its code and i still have no idea what it does


resizing:
 - impl
 - sync problem try changing move() in the end_frame() or current() -> next() in some cases 


make block size controllable
 - :depends on shader jit (spec consts)
same for world size

allow custom grass shapes:
 - what if you want thin grass rectangle or a very small batch of grass?


report flame moving timestamps around

sample light differently for more glossy surfaces

investigate typed lumal_buffers, pipelines, etc.
arena of voxel data instead of per-mapcall image bind (bind less)
 - uniform voxelArena
 - push_constants {ivec3 shift, ivec3 size} 
 - free sprite sheets

instead of extracting roughness in subpass, rasterize with stencil mask:
 - this needs separate vertices for reflective / non-reflective
  - just separate and mesh as two meshes
  - one more pipeline, same subpass?
 - this also is completely redundant if im going to have blurred reflections instead of no reflections for high roughness

2d sprites:
 - raster to 3d voxels
 - so they have materials!
 - actually, that sucks. You now have to either make pixel editors support materials, or implement a converter
 - special case for renderer? Or just orient models to camera, idk
 - if no special case, then no engine code required
 - they are just 3d models then

sprite sheets:
  - basic 3d grid-aligned animations 
  - raster to 3d and put into the same arena
  - 2d sprite sheets will fit here too

no float conversions for -0.5f in fragment
 - fixed point math?

map shader to be not 4x4x4, but 64x1x1 and divide manually
 - no hw mapping for local id to pixel

destroying pool is faster, however, speed of allocating descriptors is not important for my usecase. Or is it?

rewrite block_mesh, it has too many dependencies

IMPORTANT: compiling to wasm:
  - use wasm-bindgen
  - feature for gl wasm-compatible backend
  - move it to lumal
  - vulkan as first class support since it is more explicit
    - this means that translating vulkan to gl should be "no-op" in most cases and less work in other

define all push constant structs and use sizeof for push constant sizes

structs instead of function arguments

actions build / test:
  binary sizes
  scc complexity tracking
  wgpu can be tested / benched??
  same for vk

vertex writeable storage is not supported in web

move volumetrics to forward for perfomance and quality (when projections overlap they are rendered incorrectly atm)  

more settings to comptime via types - e.g. typed images

wgpu vk write_buffer in cmdbuffer for small sizes