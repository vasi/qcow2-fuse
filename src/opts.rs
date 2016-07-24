use std::env;
use std::io::{stderr, stdout, Write};
use std::path::Path;
use std::process::exit;

use getopts;
use util::Exit;


const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

// Option parser for our filesystem.
pub struct Options {
    // Option parser.
    opts: getopts::Options,

    // Program name.
    progname: String,

    // Program arguments.
    args: Vec<String>,
}
impl Options {
    // Create a new option parser.
    pub fn new() -> Self {
        // We need to add default FUSE arguments ourselves, rust-fuse doesn't do it for us.
        let mut opts = getopts::Options::new();
        opts.optflag("h", "help", "Show this help")
            .optflag("V", "version", "Show this program's version")
            .optflag("f", "foreground", "Run in foreground")
            .optflag("d", "debug", "Run in foreground and show debug info")
            .optmulti("o", "o", "Provide a FUSE option", "OPTION");
        let mut args: Vec<String> = env::args().collect();

        // Try to get the program name from the command-line. Otherwise, just use the cargo
        // package name.
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

    // Parse the options and return matches.
    pub fn parse(&self) -> Matches {
        // Check if getopts parsing succeeds.
        let matches = match self.opts.parse(&self.args) {
            Ok(m) => m,
            Err(e) => self.error(&e.to_string()),
        };

        // Check for options that should just print info.
        if matches.opt_present("h") {
            self.usage(Exit::Ok);
        }
        if matches.opt_present("V") {
            self.version();
        }

        // Get the positional arguments.
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

        // Add -o back to each extra FUSE option.
        let opts = matches.opt_strs("o").iter().map(|s| format!("-o{}", s)).collect();

        Matches {
            qcow2: qcow2.to_owned(),
            mountpoint: mountpoint.to_owned(),
            foreground: matches.opt_present("f") || matches.opt_present("d"),
            debug: matches.opt_present("d"),
            options: opts,
        }
    }

    // Get the brief description of this program's usage.
    fn brief(&self) -> String {
        format!("{}: Mount qcow2 images\n\nUsage: {} [options] QCOW2 MOUNTPOINT",
                self.progname,
                self.progname)
    }

    // Print the version and exit.
    fn version(&self) -> ! {
        println!("{} version {}", self.progname, VERSION);
        exit(Exit::Ok as i32);
    }

    // Print this program's usage and exit.
    fn usage(&self, code: Exit) -> ! {
        let mut f: Box<Write> = if code == Exit::Ok {
            Box::new(stdout())
        } else {
            Box::new(stderr())
        };
        writeln!(f, "{}", self.opts.usage(&self.brief())).unwrap();
        exit(code as i32);
    }

    // Print an error and exit.
    fn error(&self, message: &str) -> ! {
        writeln!(stderr(), "Error: {}\n\n", message).unwrap();
        self.usage(Exit::Error);
    }
}

// The results of option parsing.
pub struct Matches {
    // The path to mount at, possibly relative.
    pub mountpoint: String,

    // The qcow2 path to mount, possibly relative.
    pub qcow2: String,

    // Whether to mount in the foreground.
    pub foreground: bool,

    // Whether to print debug messages.
    pub debug: bool,

    // Extra -o options to pass through to FUSE.
    pub options: Vec<String>,
}
