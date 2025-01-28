is Option<u64> optimized in the same way as Option<* ()>?
is LLVM unrolling (any) loops into this in Rust: 
- for each 4 items:
    - do something
- for the remaining items:
    - do something
?
How to make code more SIMD friendly (aliasing)?

what i do not like about Rust:
- code is empty:
    - you want to find what function does
    - you end up jumping all over the place 10 minutes seing only some supergeneric soup that has 0 logic

    - I am missing that from C, where you typically open src and its immediately readable logic
- "auto" types everywhere (let):
    - you end up having no fucking idea what is going on
- traits are too complicated for my brain
    - i guess i should use macros
    - how tf does virtual dispatch work?
    - templates are just easier
- no good variadic generics (like Lum::ECManager)
- people try to make things "Rusty"
    - i still have not found good vulkan wrapper that just wraps vulkan. This is fucking insane
        - please, dont rename things
        - please, dont rename things in a fucking different way
        - please, dont change how functions work under the same name


what i do like about Rust:
- tools
    - code just compiles and i still cant believe that
    - libraries just work
    - rus- analyzer is the best lsp i have ever seen
        - except for macros
- enums
- default > constructor. I Fucking Love This
- C/C++ interop possible
- a lot of popular "default" solutions are fast
- a lot of things that let me breathe
    - black_box
    - &[a,b,c] // stack slice !
    - int overflow without dancing with the compiler
    - bounds checking without asan (i still am uncapable of running it lol)
    - $env:RUST_BACKTRACE=1; cargo run

overall, Rust is C for me and i wanted to get back to C for a long time

and what i do might actually be useful here (at least, i have seen people using other's renderers in Rust. There are even youtube videos of people doing it. And my C++ project is likely to never be used after so much work-  but to be fair, its because even after 100h of build systems it still does not compile everywhere, unlike Rust (appox. 2h of build systems))

unknown:
- compiler output is not much better than C++
- same codestyle for everyone destroys code personalization
- pointers casting kinda sucks (needed for vulkan)
- i was casting *Option<SomeType> and it was too late when i realized
- 

damn every library in rust is written by idiots
- missing key features
- rename for the sake of rename
- changes api every 1/30 version
- im not spending 1 day resolving linking
- im spending 1 day finding working crate

vulkanalia:
- names are like vulkan, but
    - 10% chance removed prefix
    - 50% chance namespace is useless (and harmful!)
        - vk::someflag::value is not better than VK_SOMEFLAG_VALUE if you cant use vk::someflag
            - why renaming then?
    - 50% chance to use builders
    - 80% chance to use slices (and 20% chance to use pointers. Im not even angry anymore, im done)

i think there is plenty of people who all need the same lib
- why dont we cooperate to create the best version of it?
- like, @i need good magicavoxel loader
    -   @i need it too!
    -   @lets build it together

Rust does have frustrating linking problems
- cardo add lib
- cargo build
- fails for different reasons