# xsynth-clib
C/C++ bindings for [XSynth](https://github.com/BlackMIDIDevs/xsynth) allowing its use in languages outside of Rust.

## How To Use
The latest release of this library for Linux, macOS and Windows, as well as the header file can be found in the [releases](https://github.com/BlackMIDIDevs/xsynth-clib/releases) section.

If you wish to use it in another platform (eg. Android, iOS, BSD) or architecture from the ones provided, you have to build the library yourself.
See instructions for building below.

## Building
First, clone the version of the library you want to use by running `git clone --branch <version> https://github.com/BlackMIDIDevs/xsynth-clib` or the latest version by simply running `git clone https://github.com/BlackMIDIDevs/xsynth-clib`.

Then considering [Rust](https://rustup.rs/) is installed on your system, you build the library using `cargo build --release`. The header file (`xsynth.h`) will be generated in the root directory of the project, while the library itself will be under the `target/release` directory.

For cross-compilation, please visit the [official Rust documentation](https://rust-lang.github.io/rustup/cross-compilation.html).

## Documentation
All necessary documentation can be found on the `xsynth.h` header file.

## License
The XSynth Rust library as well as this C library are both licensed under the MIT license.
