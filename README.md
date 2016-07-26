# qcow2-fuse

This program allows you to mount [qcow2](https://en.wikipedia.org/wiki/Qcow) virtual disk images as [FUSE](https://github.com/libfuse/libfuse) filesystems. 

* [Usage](#usage)
    * [Using the mounted image](#using-the-mounted-image)
    * [Partitions](#partitions)
    * [Options](#options)
* [Installation](#installation)
* [Features](#features)
* [License](#license)
* [See also](#see-also)

## Usage

Mount a `myimage.qcow2` onto the directory `mnt` with:

```sh
qcow2-fuse myimage.qcow2 mnt
```

The directory will now contain a file `mnt/myimage` that lets you read the virtual disk contents.

### Using the mounted image

Your virtual disk may contain just one partition, in which case you can mount it like a device:

```sh
mount mnt/myimage /mnt/myimage

# Or with another FUSE filesystem:
ext4fuse mnt/myimage /mnt/myimage
```

By default, FUSE filesystems are only available to the current user. If you want to mount the virtual filesystem as root, you'll need to pass the `-o allow_root` option to qcow2-fuse.

### Partitions

Sometimes your virtual disk contains multiple partitions, so you can't just mount it directly. Instead, ask your OS to take care of reading the partitions:

```sh
# On Linux:
kpartx -a mnt/myimage

# On macOS
hdiutil attach -imagekey diskimage-class=CRawDiskImage \
  -nomount mnt/myimage
```

This will generate new entries in `/dev` that you can then mount as above.

### Options

Several options to this program are available. You can see descriptions of some of them by running `qcow2-fuse --help`.

Many options starting with `-o` will be passed through to FUSE. You can read about these options [for Linux](http://manpages.ubuntu.com/manpages/xenial/man8/mount.fuse.8.html) and [for macOS](https://github.com/osxfuse/osxfuse/wiki/Mount-options).

## Installation

This program is written in Rust, and links to libfuse.

...

## Features

This program can mount only certain qcow2 images:

* Only version 3 (aka "qcow2 1.1") is supported; version 2 is not supported.
* This program provides read-only access, writing is not supported.
* Compressed blocks are not supported.
* Encryption is not supported.
* Backing files are not supported.
* It's ok if the image contains snapshots. But this program will only provide access to the main image, not the snapshots.
* Repairing damaged images is not supported.


## License

This program is available under the [MIT license](MIT-LICENSE).

## See also

* [qcow2](https://en.wikipedia.org/wiki/Qcow)
* [FUSE](https://github.com/libfuse/libfuse)
* [FUSE for OS X](https://osxfuse.github.io/)
* [Rust](https://www.rust-lang.org)
* [ext4fuse](https://github.com/gerard/ext4fuse)
* [kpartx](http://manpages.ubuntu.com/manpages/xenial/man8/kpartx.8.html)
