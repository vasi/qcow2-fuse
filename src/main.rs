#[macro_use]
extern crate log;

extern crate daemonize;
extern crate fuse;
extern crate getopts;
extern crate libc;
extern crate positioned_io;
extern crate qcow2;
extern crate time;

mod fs;

use std::env::args_os;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::result::Result;

use daemonize::Daemonize;
use getopts;
use fuse::{Session, Filesystem};
use qcow2::Qcow2;

use self::fs::{ReadAtFs, md_to_attrs};


const EXIT_OK: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_USAGE: i32 = 2;


// fn or_die<T, E: Display>(code: i32, msg: &str, r: Result<T, E>) -> T {
//     match r {
//         Ok(t) => t,
//         Err(e) => {
//             if !msg.is_empty() {
//                 write!(stderr(), "{}: ", msg).unwrap();
//             }
//             writeln!(stderr(), "{}", e).unwrap();
//             exit(code);
//         }
//     }
// }

// struct Args {
//     qcow2: PathBuf,
//     mountpoint: PathBuf,
//     foreground: bool,
// }
// fn parse_args() -> Result<Args, Box<Error>> {
//     let mut args = args_os().skip(1);
//     let qcow2 = try!(args.next().ok_or("No qcow2 path provided"));
//     let mountpoint = try!(args.next().ok_or("No mountpoint provided"));
//     Ok(Args {
//         qcow2: From::from(qcow2),
//         mountpoint: From::from(mountpoint),
//     })
// }
//
// fn mount<FS: Filesystem, P: AsRef<Path>>(filesystem: FS,
//                                          mountpoint: &P,
//                                          foreground: bool) {
//     let mut sess = Session::new(filesystem, mountpoint.as_ref(), &[]);
//     if !foreground {
//         let daemonize = Daemonize::new().working_directory("/");
//         die_unless(EXIT_ERROR, "Daemonizing failed", daemonize.start());
//     }
//     sess.run();
// }



fn usage(code: i32, progname: &String, opts: &Options) -> ! {
    let brief = format!("{}: Mount qcow2 images\n\nUsage: {} [options] QCOW2 MOUNTPOINT\n",
        progname, progname);
    println!("{}", opts.usage(&brief));
    exit(code);
}

fn version(progname: &String) -> ! {
    println!("{} version {}", progname, env!("CARGO_PKG_VERSION"));
    exit(EXIT_OK);
}

fn main() {
    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this help")
        .optflag("V", "version", "Show this program's version")
        .optflag("f", "foreground", "Run in foreground")
        .optflag("d", "debug", "Run in foreground and show debug info")
        .optmulti("o", "o", "Provide a FUSE option", "OPTION");

    let args: Vec<String> = std::env::args().collect();
    let progname = args[0].clone();

    if args.len() == 1 {
        usage(EXIT_USAGE, &progname, &opts);
    }
    let matches = match opts.parse(&args[1..]) {
        Err(e) => { println!("{}\n", e.to_string()); usage(EXIT_USAGE, &progname, &opts); },
        Ok(m) => m,
    };
    if matches.opt_present("h") {
        usage(EXIT_OK, &progname, &opts);
    }
    if matches.opt_present("V") {
        version(&progname);
    }
    let (qcow2, mountpoint) = match matches.free.as_slice() {
        [q, m] => (q, m),
        _ => { println!("Two arguments required"); usage(EXIT_USAGE, &progname, &opts); }
    }


    println!("free: {:?}", matches.free);
    println!("d: {:?}", matches.opt_present("d"));
    println!("V: {:?}", matches.opt_present("V"));
    println!("o: {:?}", matches.opt_strs("o"));
    println!("{}", opts.usage("header"));

//     let args = die_unless(EXIT_USAGE, "", parse_args());
//     let name = die_unless(EXIT_ERROR,
//                           "",
//                           args.qcow2.file_stem().ok_or("No filename found"));
//     let file = die_unless(EXIT_ERROR, "Error opening file", File::open(&args.qcow2));
//     let md = die_unless(EXIT_ERROR, "Failed to get file attributes", file.metadata());
//     let qcow2 = die_unless(EXIT_ERROR, "Error opening qcow2 file", Qcow2::open(file));
//     let reader = die_unless(EXIT_ERROR, "Can't get qcow2 guest reader", qcow2.reader());
//     let fs = ReadAtFs {
//         read: reader,
//         name: From::from(name),
//         attr: md_to_attrs(md),
//     };
//
//     mount(fs, &args.mountpoint, false);
}
