#[macro_export]
macro_rules! logln {
    () => {
        println!()
    };
    ($($arg:tt)*) => {
        println!("[{}]: {}", module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! elogln {
    () => {
        println!()
    };
    ($($arg:tt)*) => {
        eprintln!("[{}]: {}", module_path!(), format_args!($($arg)*))
    };
}
