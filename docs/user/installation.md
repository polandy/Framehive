# Installation

Framehive runs on Linux and macOS. It links two native libraries: LibRaw, to read RAW
files, and libgphoto2, to import from cameras and phones over PTP or MTP. The Nix flake
of the repository brings both, together with the pinned Rust toolchain, so Nix is the
supported way to install and build it. There are no prebuilt downloads yet.

## With Nix

Run it from a checkout without installing anything:

```sh
nix run . -- --help
nix run . -- import <source> <destination>
```

Or install the `framehive` command into your profile, and update it later from a newer
checkout the same way:

```sh
nix profile install .
framehive --version
```

## Building from source

`cargo` alone is not enough: outside the Nix shell, the build stops because it cannot
find LibRaw or libgphoto2. Open the shell first, then build or run:

```sh
nix develop          # or: direnv allow
cargo build --locked --release -p framehive-cli
target/release/framehive --help
```

`nix develop -c cargo run --locked -p framehive-cli -- --help` does the same in one
line.

## Cameras on Linux

To import from a camera or phone, Framehive must be able to open its USB device. Two
things commonly stand in the way:

- the desktop has already mounted the camera (GNOME's gvfs, KDE's kio): unmount it in
  the file manager, or quit the program that holds it, then run `framehive cameras`
  again;
- your user may not open the device: the udev rules that come with libgphoto2 grant
  it to the logged-in user on most distributions.

A phone shows its storage only while it is unlocked and set to file transfer.
