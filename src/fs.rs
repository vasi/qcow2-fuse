use std::fs::Metadata;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::result::Result;

use fuse::{FileAttr, Filesystem, FileType, FUSE_ROOT_ID, ReplyAttr, ReplyData, ReplyDirectory,
           ReplyEntry, Request};
use libc::{c_int, EIO, ENOENT, EPIPE};
use positioned_io::{ReadAt, Size};
use time::Timespec;


// The inodes for our only directory and our file.
const INO_DIR: u64 = FUSE_ROOT_ID;
const INO_FILE: u64 = 2;

// Stat never changes!
const TTL: Timespec = Timespec {
    sec: 1E+9 as i64,
    nsec: 0,
};

const BLOCKSIZE: u64 = 512;

pub fn md_to_attrs(md: Metadata) -> FileAttr {
    FileAttr {
        ino: 1,
        size: 0,
        blocks: 0,
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
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

pub struct ReadAtFs<I: ReadAt + Size> {
    pub read: I,
    pub name: PathBuf,
    // Base attributes.
    pub attr: FileAttr,
}
impl<I: ReadAt + Size> ReadAtFs<I> {
    fn errcode(err: io::Error) -> c_int {
        match err.raw_os_error() {
            Some(i) => i,
            None => EIO,
        }
    }
    fn file_attrs(&self) -> Result<FileAttr, c_int> {
        let size = match self.read.size() {
            Ok(Some(size)) => size,
            Ok(None) => return Err(EPIPE),
            Err(e) => return Err(Self::errcode(e)),
        };

        let mut blocks = size / BLOCKSIZE;
        if size % BLOCKSIZE > 0 {
            blocks += 1;
        }

        let mut attr = self.attr;
        attr.ino = INO_FILE;
        attr.size = size;
        attr.blocks = blocks;
        attr.kind = FileType::RegularFile;
        attr.perm = 0o644;
        attr.nlink = 1;
        Ok(attr)
    }
}

impl<I: ReadAt + Size> Filesystem for ReadAtFs<I> {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &Path, reply: ReplyEntry) {
        if parent == INO_DIR && name == self.name {
            match self.file_attrs() {
                Ok(attrs) => reply.entry(&TTL, &attrs, 0),
                Err(i) => reply.error(i),
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            INO_DIR => reply.attr(&TTL, &self.attr),
            INO_FILE => {
                match self.file_attrs() {
                    Ok(attrs) => reply.attr(&TTL, &attrs),
                    Err(i) => reply.error(i),
                }
            }
            _ => reply.error(ENOENT),
        }
    }

    fn read(&mut self,
            _req: &Request,
            ino: u64,
            _fh: u64,
            offset: u64,
            size: u32,
            reply: ReplyData) {
        if ino == INO_FILE {
            let mut buf = vec![0; size as usize];
            match self.read.read_at(offset, &mut buf) {
                Err(e) => reply.error(Self::errcode(e)),
                Ok(size) => reply.data(&buf[..size]),
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self,
               _req: &Request,
               ino: u64,
               _fh: u64,
               offset: u64,
               mut reply: ReplyDirectory) {
        if ino == INO_DIR {
            if offset == 0 {
                reply.add(INO_DIR, 0, FileType::Directory, ".");
                reply.add(INO_DIR, 1, FileType::Directory, "..");
                reply.add(INO_FILE, 2, FileType::RegularFile, &self.name);
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
}
