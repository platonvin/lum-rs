#![feature(stmt_expr_attributes)]
#![feature(custom_inner_attributes)]
#![feature(optimize_attribute)]
#![feature(where_clause_attrs)]
#![allow(unused)]
// This is a glue-file
// files that start with "all_" are initializing / destroying resources (packed in structs)

pub mod containers;
pub mod renderer;

// this is basically safier version of assert! that is checked in debug mode
// in release mode opens into just assume!
// std::intrinsics::assume
#[macro_export]
macro_rules! assert_assume {
    ($cond:expr) => {{
        // Do runtime checks in debug mode
        debug_assert!($cond);
        // but also provide assumption to the compiler
        unsafe {
            std::hint::assert_unchecked($cond);
        }
    }};
}

#[macro_export]
macro_rules! assert_unreachable {
    () => {
        if cfg!(debug_assertions) {
            // In debug mode, verify that the code never executes
            panic!();
        } else {
            unreachable!();
        }
    };
}

#[macro_export]
macro_rules! for_zyx {
    // Handle ivec3 argument with a closure
    ($dims:expr, $body:expr) => {
        for zz in 0..$dims.z {
        for yy in 0..$dims.y {
        for xx in 0..$dims.x {
            $body(xx, yy, zz)
        }}}
    };

    // Handle 3 separate integers with a closure
    ($x_dim:expr, $y_dim:expr, $z_dim:expr, $body:expr) => {
        for zz in 0..$z_dim {
        for yy in 0..$y_dim {
        for xx in 0..$x_dim {
            $body(xx, yy, zz)
        }}}
    };
}
