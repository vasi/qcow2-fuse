use std::env;
use std::io::{stderr, stdout, Write};
use std::path::Path;
use std::process::exit;

use getopts;
use util::Exit;


const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct Options {
    opts: getopts::Options,
    progname: String,
    args: Vec<String>,
}
impl Options {
    pub fn new() -> Self {
        let mut opts = getopts::Options::new();
        opts.optflag("h", "help", "Show this help")
            .optflag("V", "version", "Show this program's version")
            .optflag("f", "foreground", "Run in foreground")
            .optflag("d", "debug", "Run in foreground and show debug info")
            .optmulti("o", "o", "Provide a FUSE option", "OPTION");
        let mut args: Vec<String> = env::args().collect();

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

    pub fn parse(&self) -> Matches {
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

pub struct Matches {
    pub mountpoint: String,
    pub qcow2: String,
    pub foreground: bool,
    pub debug: bool,
    pub options: Vec<String>,
}
