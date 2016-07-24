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
mod fuse_util;
mod opts;
mod util;

use std::ffi::OsStr;
use std::fs::{File, metadata};
use std::path::Path;

use qcow2::Qcow2;

use fs::ReadAtFs;
use util::{error, OrDie};
use opts::Options;


fn main() {
    let matches = Options::new().parse();
    util::set_logger(matches.debug);

    // Check that the mountpoint looks ok.
    match metadata(&matches.mountpoint) {
        Err(e) => error(&format!("Can't access mountpoint: {}", e)),
        Ok(ref m) if !m.is_dir() => error("Mountpoint must be a directory"),
        _ => {}
    }

    // Get the basename of the qcow file.
    let path = Path::new(&matches.qcow2);
    let mut name = path.file_name().unwrap_or(OsStr::new("disk"));
    if path.extension() == Some(OsStr::new("qcow2")) {
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
        attr: fuse_util::md_to_attrs(md),
        foreground: matches.foreground,
    };
    fuse_util::mount(fs, &matches.mountpoint, matches.foreground, matches.options);
}
