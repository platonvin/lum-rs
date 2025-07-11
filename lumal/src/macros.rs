//! module with some macros used by lum and lumal

/// prints current function in stdio
#[macro_export]
macro_rules! atrace {
    () => {
        println!("\x1b[32m{}:{}: Fun: {}\x1b[0m", file!(), line!(), {
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            let name = type_name_of(f);
            name.strip_suffix("::f").unwrap()
        });
    };
}
