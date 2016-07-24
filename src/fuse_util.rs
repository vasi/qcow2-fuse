use std::ffi::OsStr;
use std::fs::Metadata;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::ptr::null_mut;

use daemonize::Daemonize;
use fuse::{BackgroundSession, FileAttr, Filesystem, FileType, Session};
use libc::{c_int, EIO, pthread_sigmask, SIG_BLOCK, SIGINT, sigemptyset, sigaddset, sigset_t, sigwait};
use time::Timespec;

use util::OrDie;

// Create a fuse::FileAttr based on a std::fs::Metadata.
pub fn md_to_attrs(md: Metadata) -> FileAttr {
    FileAttr {
        // These items are appropriate for the directory, but will be overwritten for any files.
        ino: 1,
        size: 0,
        blocks: 0,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        // These items will be reused.
        rdev: 0,
        flags: 0,
        atime: Timespec::new(md.atime(), md.atime_nsec() as i32),
        mtime: Timespec::new(md.mtime(), md.mtime_nsec() as i32),
        ctime: Timespec::new(md.ctime(), md.ctime_nsec() as i32),
        crtime: Timespec::new(md.ctime(), md.ctime_nsec() as i32),
        uid: md.uid(),
        gid: md.gid(),
    }
}

// Mount a filesystem in the foreground.
//
// This is complicated, because we want to listen for Control-C.
fn run_foreground<FS: Filesystem + Send>(sess: Session<FS>) {
    // Block signals on all threads.
    let mut sigset: sigset_t = 0 as sigset_t;
    let sigsetp: *mut sigset_t = &mut sigset;
    unsafe {
        sigemptyset(sigsetp);
        sigaddset(sigsetp, SIGINT);
        pthread_sigmask(SIG_BLOCK, sigsetp, null_mut());
    }

    // Start a background thread to run the filesystem.
    let background: BackgroundSession = unsafe { sess.spawn() }.or_die("Can't spawn session");

    // Wait for SIGINT.
    loop {
        let mut sig: c_int = 0;
        let sigp: *mut c_int = &mut sig;
        let e = unsafe { sigwait(sigsetp, sigp) };
        if e == 0 && sig == SIGINT {
            // Break the loop, drop the guard, and unmount.
            break;
        }
    }

    // Just a reminder that the filesystem dies here.
    drop(background);
}

// Mount a filesystem in the background (daemonized).
fn run_background<FS: Filesystem + Send>(mut sess: Session<FS>) {
    // Daemonize, then run.
    let daemonize = Daemonize::new().working_directory("/");
    daemonize.start().or_die("Failed to daemonize");
    sess.run();
}

// Mount a filesystem at a path.
pub fn mount<FS: Filesystem + Send, P: AsRef<Path>>(filesystem: FS,
                                                mountpoint: &P,
                                                foreground: bool,
                                                options: Vec<String>) {
    // Build the opt array. Include fuse options starting with -o.
    let opts: Vec<String> = options.iter().map(|s| format!("-o{}", s)).collect();
    let opts: Vec<&OsStr> = opts.iter().map(|s| OsStr::new(s)).collect();
    debug!("FUSE options: {:?}", opts);

    // Setup the session now, before any chroot.
    let sess = Session::new(filesystem, mountpoint.as_ref(), opts.as_slice());
    if foreground {
        run_foreground(sess);
    } else {
        run_background(sess);
    }
}

// Turn an I/O error into something FUSE can understand.
pub fn fuse_errcode(err: io::Error) -> c_int {
    match err.raw_os_error() {
        Some(i) => i,
        None => EIO,
    }
}
