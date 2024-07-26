# xsynth-clib
A dynamic C/C++ library for XSynth allowing its use in languages outside of Rust.

## How to use
To use this you need to build the library by running `cargo build --release`. A header file `xsynth.h` will be generated in the root directory which can be used in your code.

Then you will find the XSynth dynamic library file under `./target/release` (should be `libxsynth.so` for Linux and `xsynth.dll` for Windows) and feed it to your compiler.

## Documentation?
All necessary documentation can be found on the generated `xsynth.h` header file.

## What is implemented?
Implemented:
- Channel Group
- Sample Soundfont

Not yet implemented:
- Realtime module
- Rendered module

## License
The XSynth Rust library as well as this C library are both licensed under the LGPL 3.0
