//! Contract tests for the Rust build standard.
//!
//! The standard makes the parallel `rustc` frontend and, on Linux, mold the
//! default for every development build. Cargo applies one `rustflags` source
//! rather than merging them, and an assigned `RUSTFLAGS` replaces every source,
//! so the flags are repeated in each configuration source, restated by each
//! development recipe, and kept out of the coverage and release recipes. The
//! Makefile clauses read what `make -n` prints on a Linux and a macOS host; each
//! listed `setup-rust` step passes `install-mold`. Fixtures come first, so no
//! rule passes by detecting nothing.

#[path = "build_standard_support/chain_exhaustive.rs"]
mod chain_exhaustive;
#[path = "build_standard_support/ci_steps.rs"]
mod ci_steps;
#[path = "build_standard_support/command_reader.rs"]
mod command_reader;
#[path = "build_standard_support/config.rs"]
mod config;
#[path = "build_standard_support/coverage_env.rs"]
mod coverage_env;
#[path = "build_standard_support/cranelift.rs"]
mod cranelift;
#[path = "build_standard_support/development.rs"]
mod development;
#[path = "build_standard_support/exhaustive.rs"]
mod exhaustive;
#[path = "build_standard_support/fixtures.rs"]
mod fixtures;
#[path = "build_standard_support/injected.rs"]
mod injected;
#[path = "build_standard_support/injected_held_out.rs"]
mod injected_held_out;
#[path = "build_standard_support/make.rs"]
mod make;
#[path = "build_standard_support/policy_cases.rs"]
mod policy_cases;
#[path = "build_standard_support/process.rs"]
mod process;
#[path = "build_standard_support/reader_cases.rs"]
mod reader_cases;
#[path = "build_standard_support/shell.rs"]
mod shell;
#[path = "build_standard_support/workflow_exhaustive.rs"]
mod workflow_exhaustive;
use ci_steps::workflow_problems;
use config::{CONFIG, Pin, Problems, TOOLCHAIN, config_problems};
use coverage_env::coverage_recipe_problems;
use development::development_problems;
use make::{Host, Target, held_out_problems, held_out_target_count};
use process::real_make;

/// Turns a list of complaints into a test result.
fn none_of(problems: &Problems) -> Result<(), String> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("{problems:#?}"))
    }
}

/// Every workflow that builds under the standard installs mold. A repository
/// whose workflows do not set up Rust through `setup-rust` lists none, and the
/// check then reads nothing; a listed workflow must have a step to read.
#[test]
fn every_setup_rust_step_installs_linker() -> Result<(), String> {
    none_of(&workflow_problems())
}

#[test]
fn every_rustflags_source_is_consistent_with_the_pin() -> Result<(), String> {
    none_of(&config_problems(CONFIG, Pin::read(TOOLCHAIN)?)?)
}

#[test]
fn development_targets_restate_the_flags_on_linux() -> Result<(), String> {
    let (problems, read) = development_problems(real_make, Host::Linux, Pin::read(TOOLCHAIN)?)?;
    none_of(&problems)?;
    if read == 0 {
        return Err(
            "no development target assigns RUSTFLAGS, so the check proves nothing".to_owned(),
        );
    }
    Ok(())
}

#[test]
fn development_targets_keep_the_frontend_but_not_the_linker_elsewhere() -> Result<(), String> {
    none_of(&development_problems(real_make, Host::Darwin, Pin::read(TOOLCHAIN)?)?.0)
}

/// Coverage measures and release ships, so both stay on the default flags. A
/// repository that lists no such target has nothing local to hold out, and the
/// check then reads no commands; otherwise it must read at least one.
#[test]
fn coverage_and_release_take_neither_flag() -> Result<(), String> {
    let (problems, read) = held_out_problems(real_make)?;
    none_of(&problems)?;
    if held_out_target_count() > 0 && read == 0 {
        return Err(
            "the held-out targets run no cargo command, so the check proves nothing".to_owned(),
        );
    }
    Ok(())
}

/// The coverage recipe assigns what it recorded besides `RUSTFLAGS`: the LLVM backend, the linker and the
/// link arguments a measurement needs. A repository with no local coverage recipe records nothing.
#[test]
fn the_coverage_recipe_assigns_its_recorded_environment() -> Result<(), String> {
    none_of(&coverage_recipe_problems(real_make)?)
}

/// Returns the command `make -n` prints for `lint-whitaker` on a host: the line whose program is the Whitaker
/// executable, found by its words rather than by a substring.
fn whitaker_command(host: Host) -> Result<String, String> {
    let recipe = real_make(Target("lint-whitaker"), host)?;
    recipe
        .lines()
        .find(|line| {
            line.split_whitespace()
                .any(|word| shell::names_program(word, "whitaker"))
        })
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "`make lint-whitaker` on {} must run the Whitaker executable",
                host.make_value()
            )
        })
}

/// Whitaker builds with its own installer-managed toolchain, so its route clears every inherited flag,
/// backend and linker setting instead of restating the standard, and passes the warning policy through
/// Dylint's own input. It neither assigns `RUSTFLAGS` nor selects LLVM, so Cargo's repository defaults
/// stay in force for the checked crate. The route runs on Linux and macOS hosts, so both are read and each
/// clears its own target settings.
#[test]
fn whitaker_clears_inherited_settings_and_takes_the_repository_defaults() -> Result<(), String> {
    let mut problems = Problems::new();
    for host in [Host::Linux, Host::Darwin] {
        let whitaker = whitaker_command(host)?;
        for setting in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "CARGO_BUILD_TARGET",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER",
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS",
            "CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER",
            "CARGO_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS",
            "CFLAGS",
            "LDFLAGS",
        ] {
            if !whitaker.contains(&format!("-u {setting} ")) {
                problems.push(format!(
                    "Whitaker on {} must remove inherited {setting}: {whitaker}",
                    host.make_value()
                ));
            }
        }
        if whitaker.contains(" RUSTFLAGS=\"") || whitaker.contains("=llvm") {
            problems.push(format!(
                "Whitaker must not replace Cargo defaults with assigned flags or LLVM: {whitaker}"
            ));
        }
        if !whitaker.contains("DYLINT_RUSTFLAGS=\"-D warnings") {
            problems.push(format!(
                "warnings must reach the check through Dylint's input: {whitaker}"
            ));
        }
        // Other Cargo options may follow (`--locked`), so require both selections rather than a suffix.
        for option in ["--all-targets", "--all-features"] {
            if !whitaker.split_whitespace().any(|word| word == option) {
                problems.push(format!(
                    "the check must keep {option}, which is how Whitaker reaches every target: {whitaker}"
                ));
            }
        }
    }
    none_of(&problems)
}

/// `make lint` runs both halves, so isolating Whitaker cannot drop either from the gate: one command runs
/// Clippy through Cargo and another runs the Whitaker executable.
#[test]
fn lint_runs_clippy_and_whitaker() -> Result<(), String> {
    let recipe = real_make(Target("lint"), Host::Linux)?;
    let runs = |program: &str, subcommand: Option<&str>| {
        recipe.lines().any(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words.iter().any(|word| shell::names_program(word, program))
                && subcommand.is_none_or(|sub| words.contains(&sub))
        })
    };
    let mut problems = Problems::new();
    if !runs("cargo", Some("clippy")) {
        problems.push(format!(
            "`make lint` no longer runs `cargo clippy`: {recipe}"
        ));
    }
    if !runs("whitaker", None) {
        problems.push(format!("`make lint` no longer runs Whitaker: {recipe}"));
    }
    none_of(&problems)
}
