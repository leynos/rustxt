//! Contract that the release workflow builds without the build standard's flags.
//!
//! A release ships, so it stays on the platform linker and the stable
//! toolchain: `cross +stable` refuses the nightly-only `-Zthreads`, and an
//! assigned `RUSTFLAGS` (even a plain `-D warnings`) displaces every
//! `rustflags` source in `.cargo/config.toml`. The workflow is read at compile
//! time, so the test needs no filesystem access.

const RELEASE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/release.yml"
));

/// Returns the lines of the step that holds a release build command.
fn release_build_step() -> Vec<&'static str> {
    let lines: Vec<&str> = RELEASE.lines().collect();
    let Some(at) = lines
        .iter()
        .position(|line| line.contains("build --release"))
    else {
        return Vec::new();
    };
    let start = lines
        .iter()
        .take(at)
        .rposition(|line| line.trim_start().starts_with("- name:"))
        .unwrap_or(0);
    lines
        .iter()
        .skip(start)
        .take(at - start + 1)
        .copied()
        .collect()
}

/// Scenario: the release workflow's build step.
///
/// Invariant: it assigns `RUSTFLAGS` itself, and the value names neither the
/// parallel frontend nor mold.
#[test]
fn the_release_build_assigns_rustflags_without_the_standard_flags() {
    let step = release_build_step();
    assert!(
        !step.is_empty(),
        "the release workflow no longer builds with `build --release`"
    );
    let assigned: Vec<&str> = step
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with("RUSTFLAGS:"))
        .collect();
    let [value] = assigned.as_slice() else {
        panic!("the release build must assign RUSTFLAGS once: {step:?}");
    };
    assert!(
        !value.contains("-Zthreads") && !value.contains("mold"),
        "the release takes a standard flag: {value}"
    );
}
