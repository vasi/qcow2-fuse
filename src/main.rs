#[macro_use]
extern crate log;

extern crate daemonize;
extern crate env_logger;
extern crate fuse;
extern crate libc;
extern crate positioned_io;
extern crate qcow2;
extern crate time;

mod fs;

use std::env::args_os;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::{stderr, Write};
use std::path::PathBuf;
use std::process::exit;
use std::result::Result;

use daemonize::Daemonize;
use qcow2::Qcow2;

use self::fs::{ReadAtFs, md_to_attrs};


const EXIT_USAGE: i32 = 2;
const EXIT_ERROR: i32 = 1;


fn die_unless<T, E: Display>(code: i32, msg: &str, r: Result<T, E>) -> T {
    match r {
        Ok(t) => t,
        Err(e) => {
            if !msg.is_empty() {
                write!(stderr(), "{}: ", msg).unwrap();
            }
            writeln!(stderr(), "{}", e).unwrap();
            exit(code);
        }
    }
}

struct Args {
    qcow2: PathBuf,
    mountpoint: PathBuf,
}
fn parse_args() -> Result<Args, Box<Error>> {
    let mut args = args_os().skip(1);
    let qcow2 = try!(args.next().ok_or("No qcow2 path provided"));
    let mountpoint = try!(args.next().ok_or("No mountpoint provided"));
    Ok(Args {
        qcow2: From::from(qcow2),
        mountpoint: From::from(mountpoint),
    })
}

use std::fs::OpenOptions;
use std::sync::{Mutex};
use std::path::{Path};
use std::ffi::OsStr;
use log::{Log, LogMetadata, LogRecord, LogLevelFilter};
use fuse::{Session, Filesystem};

struct Logger(pub Mutex<File>);
impl Log for Logger {
    fn enabled(&self, _metadata: &LogMetadata) -> bool {
        true
    }
    fn log(&self, record: &LogRecord) {
        let file = &mut self.0.lock().unwrap();
        file.write_fmt(*record.args()).unwrap();
        file.write_all(b"\n").unwrap();
        file.flush().unwrap();
    }
}

pub fn mount_daemonized<FS: Filesystem, P: AsRef<Path>> (filesystem: FS, mountpoint: &P, options: &[&OsStr]) {
    let mut sess = Session::new(filesystem, mountpoint.as_ref(), options);
    let daemonize = Daemonize::new().working_directory("/");
    die_unless(EXIT_ERROR, "Daemonizing failed", daemonize.start());
    sess.run();
}

fn main() {
    // env_logger::init().unwrap();
    let log_file = die_unless(EXIT_ERROR, "", OpenOptions::new().write(true)
        .create(true).truncate(true).open("/tmp/qcow.log"));
    let logger = Logger(Mutex::new(log_file));
    log::set_logger(|max| {
        max.set(LogLevelFilter::Trace);
        Box::new(logger)
    }).unwrap();

    let args = die_unless(EXIT_USAGE, "", parse_args());
    let name = die_unless(EXIT_ERROR,
                          "",
                          args.qcow2.file_stem().ok_or("No filename found"));
    let file = die_unless(EXIT_ERROR, "Error opening file", File::open(&args.qcow2));
    let md = die_unless(EXIT_ERROR, "Failed to get file attributes", file.metadata());
    let qcow2 = die_unless(EXIT_ERROR, "Error opening qcow2 file", Qcow2::open(file));
    let reader = die_unless(EXIT_ERROR, "Can't get qcow2 guest reader", qcow2.reader());
    let fs = ReadAtFs {
        read: reader,
        name: From::from(name),
        attr: md_to_attrs(md),
    };

    mount_daemonized(fs, &args.mountpoint, &[]);
}
