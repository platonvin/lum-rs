#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![feature(stmt_expr_attributes)]

/*
 * This is a glue-file
 * files that start with "all_" are initializing / destroying resources (packed in structs)
 * internal_renderer is where all the gpu commands are submitted
 * renderer is a wrapper around internal_renderer that is more stable and easier to use
*/

pub mod consts;
pub mod types;
pub mod containers;
pub mod internal_renderer;
pub mod renderer;

// this is basically safier version of assert! that is checked in debug mode
// in release mode opens into just assume!

#[macro_export]
macro_rules! assert_assume {
    ($cond:expr) => {
        if cfg!(debug_assertions) {
            // In debug mode, use assert! for runtime checks
            assert!($cond);
        } else {
            // In release mode, use assume to hint to the compiler
            unsafe {
                std::hint::assert_unchecked($cond);
            }
        }
    };
}