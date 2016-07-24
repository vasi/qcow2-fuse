use std::env;
use std::fmt::Display;
use std::io::{stderr, Write};
use std::process::exit;

use env_logger::LogBuilder;


// Some useful exit codes.
#[derive(PartialEq)]
pub enum Exit {
    Ok = 0,
    Error = 1,
    Usage = 2,
}

// Print an error and exit.
pub fn error(message: &str) -> ! {
    writeln!(stderr(), "{}", message).unwrap();
    exit(Exit::Error as i32);
}

// An or_die() method for Results, that prints a nice error message.
pub trait OrDie<T> {
    fn or_die(self, msg: &str) -> T;
}
impl<T, E: Display> OrDie<T> for Result<T, E> {
    fn or_die(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(e) => error(&format!("{}: {}", msg, e)),
        }
    }
}

// Set the logger to a given level.
pub fn set_logger(debug: bool) {
    let mut builder = LogBuilder::new();
    if debug {
        builder.parse("debug");
    } else if let Ok(v) = env::var("RUST_LOG") {
        builder.parse(&v);
    }
    builder.init().unwrap();
}
