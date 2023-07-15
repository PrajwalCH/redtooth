#[macro_export]
macro_rules! logln {
    () => {};
    ($($arg:tt)*) => {
        println!("[{}]: {}", module_path!(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! elogln {
    () => {};
    ($($arg:tt)*) => {
        eprintln!("[{}]: {}", module_path!(), format_args!($($arg)*));
    };
}
