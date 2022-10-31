#[macro_export]
macro_rules! debug {
    ($($val:expr),+$(,)?) => {
        if crate::DEBUG {
            $(
                println!("{:#?}", $val);
            )*
        }
    }
}

#[macro_export]
macro_rules! usage {
    ($($val:tt),+$(,)?) => {
        $(
            print!("{}", $val);
        )*

        std::process::exit(1);
    }
}
