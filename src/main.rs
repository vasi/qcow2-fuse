#[macro_use]
extern crate log;

extern crate daemonize;
extern crate env_logger;
extern crate fuse;
extern crate getopts;
extern crate libc;
extern crate positioned_io;
extern crate qcow2;
extern crate time;

mod fs;

use std::env;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{File, metadata};
use std::io::{stderr, stdout, Write};
use std::path::Path;
use std::process::exit;
use std::result::Result;

use daemonize::Daemonize;
use env_logger::LogBuilder;
use fuse::{Session, Filesystem};
use qcow2::Qcow2;

use self::fs::{ReadAtFs, md_to_attrs};


const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

#[derive(PartialEq)]
enum Exit {
    Ok = 0,
    Error = 1,
    Usage = 2,
}
fn error(message: &str) -> ! {
    writeln!(stderr(), "{}", message).unwrap();
    exit(Exit::Error as i32);
}


struct Options {
    opts: getopts::Options,
    progname: String,
    args: Vec<String>,
}
impl Options {
    fn new() -> Self {
        let mut opts = getopts::Options::new();
        opts.optflag("h", "help", "Show this help")
            .optflag("V", "version", "Show this program's version")
            .optflag("f", "foreground", "Run in foreground")
            .optflag("d", "debug", "Run in foreground and show debug info")
            .optmulti("o", "o", "Provide a FUSE option", "OPTION");
        let mut args: Vec<String> = std::env::args().collect();

        let progname = args.remove(0);
        let progname = match Path::new(&progname).file_name() {
            Some(p) => p.to_string_lossy().into_owned(),
            None => NAME.to_owned(),
        };

        Options {
            opts: opts,
            progname: progname,
            args: args,
        }
    }

    fn parse(&self) -> Matches {
        let matches = match self.opts.parse(&self.args) {
            Ok(m) => m,
            Err(e) => self.error(&e.to_string()),
        };
        if matches.opt_present("h") {
            self.usage(Exit::Ok);
        }
        if matches.opt_present("V") {
            self.version();
        }

        let mut free = matches.free.iter();
        let qcow2 = match free.next() {
            Some(q) => q,
            None => self.usage(Exit::Usage),
        };
        let mountpoint = match free.next() {
            Some(m) => m,
            None => self.error("No mountpoint provided."),
        };
        if free.next().is_some() {
            self.error("Too many arguments.");
        }

        Matches {
            qcow2: qcow2.to_owned(),
            mountpoint: mountpoint.to_owned(),
            foreground: matches.opt_present("f") || matches.opt_present("d"),
            debug: matches.opt_present("d"),
            options: matches.opt_strs("o"),
        }
    }

    fn brief(&self) -> String {
        format!("{}: Mount qcow2 images\n\nUsage: {} [options] QCOW2 MOUNTPOINT",
                self.progname,
                self.progname)
    }

    fn version(&self) -> ! {
        println!("{} version {}", self.progname, VERSION);
        exit(Exit::Ok as i32);
    }

    fn usage(&self, code: Exit) -> ! {
        let mut f: Box<Write> = if code == Exit::Ok {
            Box::new(stdout())
        } else {
            Box::new(stderr())
        };
        writeln!(f, "{}", self.opts.usage(&self.brief())).unwrap();
        exit(code as i32);
    }

    fn error(&self, message: &str) -> ! {
        writeln!(stderr(), "Error: {}\n\n", message).unwrap();
        self.usage(Exit::Error);
    }
}

struct Matches {
    mountpoint: String,
    qcow2: String,
    foreground: bool,
    debug: bool,
    options: Vec<String>,
}

fn mount<FS: Filesystem, P: AsRef<Path>>(filesystem: FS,
                                         mountpoint: &P,
                                         foreground: bool,
                                         options: Vec<String>) {
    let opts: Vec<String> = options.iter().map(|s| format!("-o{}", s)).collect();
    let opts: Vec<&OsStr> = opts.iter().map(|s| OsStr::new(s)).collect();
    debug!("FUSE options: {:?}", opts);
    let mut sess = Session::new(filesystem, mountpoint.as_ref(), opts.as_slice());
    if !foreground {
        let daemonize = Daemonize::new().working_directory("/");
        daemonize.start().or_die("Failed to daemonize");
    }
    sess.run();
}

trait OrDie<T> {
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

fn set_logger(debug: bool) {
    let mut builder = LogBuilder::new();
    if debug {
        builder.parse("debug");
    } else if let Ok(v) = env::var("RUST_LOG") {
        builder.parse(&v);
    }
    builder.init().unwrap();
}

fn main() {
    let matches = Options::new().parse();
    set_logger(matches.debug);

    // Check that the mountpoint looks ok.
    match metadata(&matches.mountpoint) {
        Err(e) => error(&format!("Can't access mountpoint: {}", e)),
        Ok(ref m) if !m.is_dir() => error("Mountpoint must be a directory"),
        _ => {},
    }

    // Get the basename of the qcow file.
    let path = Path::new(&matches.qcow2);
    let mut name = path.file_name().unwrap_or(OsStr::new("disk"));
    if path.extension() == Some(OsStr::new(".qcow2")) {
        if let Some(n) = path.file_stem() {
            name = n;
        }
    }

    // Get attributes, so we can apply them to the virtual disk.
    let file = File::open(path).or_die("Error opening qcow2 file");
    let md = file.metadata().or_die("Can't access qcow2 file");
    let qcow2 = match Qcow2::open(file) {
        Ok(q) => q,
        Err(qcow2::Error::FileType) => error("Not a qcow2 file"),
        Err(e) => error(&format!("Can't parse qcow2 file: {}", e)),
    };
    let reader = qcow2.reader().or_die("Can't get qcow2 guest reader");

    let fs = ReadAtFs {
        read: reader,
        name: From::from(name),
        attr: md_to_attrs(md),
    };
    mount(fs, &matches.mountpoint, matches.foreground, matches.options);
}

// FIXME: options
// FIXME: unmount on Ctrl-C
