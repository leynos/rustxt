# Users' guide

## Building from source on Linux

The repository's `.cargo/config.toml` selects the `mold` linker on Linux, so a
plain `cargo build --release` needs it installed.

- Install `mold`. rustc passes `-fuse-ld=mold` through its default `cc` driver,
  which must be GCC 12.1 or newer, or clang.
- If `cc` is an older GCC, install `clang` and select it:

  ```bash
  RUSTFLAGS="-Clinker=clang -Clink-arg=-fuse-ld=mold" cargo build --release
  ```

- To build with the platform linker instead, assign an empty `RUSTFLAGS`. An
  assigned `RUSTFLAGS` displaces every flag the configuration sets:

  ```bash
  RUSTFLAGS="" cargo build --release
  ```

macOS and Windows keep their platform linker; only Linux is affected.
