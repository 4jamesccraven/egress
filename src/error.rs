use std::panic::Location;

pub trait ResultExt<T> {
    fn responsible_expect(self, message: &str) -> T;
}

impl<T, E: std::fmt::Debug> ResultExt<T> for Result<T, E> {
    #[track_caller]
    fn responsible_expect(self, message: &str) -> T {
        let location = Location::caller();

        self.expect(&format!(
            "Fatal: {message}, ({}:{},{})",
            location.file(),
            location.line(),
            location.column()
        ))
    }
}
