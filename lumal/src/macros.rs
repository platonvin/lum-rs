// pub
#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }}
}

#[macro_export]
macro_rules! atrace {
    () => {
        println!(
            "\x1b[32m{}:{}: Fun: {}\x1b[0m",
            file!(),
            line!(),
            {
                fn f() {}
                fn type_name_of<T>(_: T) -> &'static str {
                    std::any::type_name::<T>()
                }
                let name = type_name_of(f);
                name.strip_suffix("::f").unwrap()
            }
        );
    };
}

pub(crate) use function;
pub(crate) use atrace;