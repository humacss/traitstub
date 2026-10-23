//! Real compiler boundaries: rejected inputs, exported contracts and feature scope.
//! Each case owns its source programs and temporary build outputs. No Cargo
//! subprocess, separate test packages or shared setup files are needed.

use rstest::rstest;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const DECLARATIVE_PROVIDER: &str = r###"
pub mod api {
    core_alias::deps! {
        export = exported_lookup;
        target = Lookup;
        scope = crate::api;
        value() -> std::path::PathBuf, || std::path::PathBuf::from("production");
    }
}

pub mod typed {
    pub struct Value(pub u64);
    core_alias::deps! {
        export = exported_declarative_types;
        scope = crate::typed;
        type Value = source::typed::Value;
        load(id: u64) -> Self::Value, |id| Value(id);
        save(value: Self::Value) -> u64, |value| value.0;
        copy[T: Clone](value: T) -> (T,T), |value| (value.clone(),value);
        trim['a](value: &'a str) -> &'a str, |value| value.trim();
    }
}
"###;

const DECLARATIVE_CONSUMER: &str = r###"
#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
use provider::api::Lookup;
core_alias::stubs! {
    target = Lookup;
    value => || std::path::PathBuf::from("shared");
}
#[test]
fn exported_contracts_need_no_procedural_metadata_or_source_imports() {
    assert_eq!(Lookup.value(), std::path::PathBuf::from("production"));
    assert_eq!(Defs.value(), std::path::PathBuf::from("shared"));
    let name = String::from("fixture");
    let deps =
        core_alias::defs! { value() -> std::path::PathBuf => || std::path::PathBuf::from(&name); };
    assert_eq!(deps.value(), std::path::PathBuf::from("fixture"));
}

#[cfg(test)]
mod typed {
    use provider::typed::Deps;
    pub struct TestValue(pub u64);
    core_alias::stubs! {
        type Value = TestValue;
        load => |id| TestValue(id+1);
        save => |value| value.0;
        copy => |value| (value.clone(), value);
        trim => |value| value.trim();
    }
    #[test]
    fn imported_schemas_and_named_defaults_keep_both_crates() {
        let calls = std::cell::Cell::new(0);
        let deps = core_alias::defs! {
            save(value: TestValue) -> u64 => |value| { calls.set(calls.get() + 1); value.0 * 2 };
            copy(value) => (value.clone(), value);
        };
        assert_eq!(deps.save(deps.load(1)), 4);
        assert_eq!(calls.get(), 1);
        assert_eq!(deps.copy(7), (7, 7));
        assert_eq!(deps.trim(" hi "), "hi");
    }
}
#[cfg(test)]
mod caller {
    use crate::typed::Defs;
    use provider::typed::Deps;
    #[test]
    fn imported_defaults_keep_their_original_scope() {
        let deps = core_alias::defs! { save(value) => value.0 * 3; };
        assert_eq!(deps.save(deps.load(1)), 6);
    }
}

#[cfg(test)]
mod exported_defaults {
    use provider::typed::Deps;
    core_alias::stubs! {
        export = declarative_exported_defaults;
        scope = crate::exported_defaults;
        load => |id| source::typed::Value(id);
        save => |value| value.0 + 10;
        copy => |value| (value.clone(), value);
        trim => |value| value;
    }
}
#[cfg(test)]
mod exported_caller {
    use crate::exported_defaults::Defs;
    use provider::typed::Deps;
    #[test]
    fn explicit_default_exports_still_preserve_the_schema_crate() {
        let deps = core_alias::defs! { load(id) => source::typed::Value(id + 1); };
        assert_eq!(deps.save(deps.load(1)), 12);
    }
}
"###;

const GENERIC_PROVIDER: &str = r###"
pub mod api {
    pub struct Input(pub u64);
    pub struct Output(pub u64);
    renamed::deps! {
        export = exported_contract;
        target = Contract;
        scope = crate::api;
        type Value = u64;
        type Error = &'static str;
        copy[U: Clone](input: U) -> (U, U), |input| (input.clone(), input);
        transform(input: source::api::Input, value: Self::Value)
            -> Result<(source::api::Output, Self::Value), Self::Error>,
            |input, value| Ok((Output(input.0 + 1), value));
        path() -> std::path::PathBuf, std::path::PathBuf::new;
        debug[U: std::fmt::Debug](value: U) -> String, |value| format!("{value:?}");
        borrow['a](input: &'a str) -> &'a str, |input| input;
    }
}

// Provider enables present; consumer enables consumer_only. Conditions must be
// resolved here, not evaluated against the consumer's unrelated feature set.
pub mod conditional {
    renamed::deps! {
        export = conditional_contract;
        scope = crate::conditional;
        #[cfg(feature = "present",)]
        enabled() -> u64, || 1;
        #[cfg(feature = "consumer_only")]
        disabled() -> u64, || 99;
        #[cfg_attr(feature = "present", cfg(all()))]
        nested() -> u64, || 2;
        #[cfg_attr(feature = "present", cfg(any()), allow(dead_code))]
        nested_disabled() -> u64, || 99;
        #[cfg_attr(not(feature = "present"), cfg(any()))]
        unconditional() -> u64, || 3;
    }
}
"###;

const GENERIC_CONSUMER: &str = r###"
#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
#[cfg(test)]
use provider::api::Contract;
renamed::stubs! {
    contract = provider::api::Contract;
    copy => |input| (input.clone(), input);
    transform => |input, value| Ok((source::api::Output(input.0 + 10), value));
    path => std::path::PathBuf::new;
    debug => |value| format!("{value:?}");
    borrow => |input| input;
}
#[test]
fn external_contracts_and_renamed_dependencies_work() {
    let deps = renamed::defs! {
        copy(input) => (input.clone(), input);
    };
    let result = deps.transform(provider::api::Input(8), 7u64).unwrap();
    assert_eq!((result.0 .0, result.1), (18, 7));
    let borrowed = String::from("borrowed");
    assert_eq!(deps.borrow(&borrowed), "borrowed");
    assert_eq!(deps.copy(5), (5, 5));
    assert!(deps.path().as_os_str().is_empty());
    assert_eq!(deps.debug(19), "19");
}

#[test]
fn qualified_contract_paths_work_for_explicit_layers() {
    let deps = renamed::defs! {
        contract = provider::api::Contract;
        target = Defs;
        copy(value) => (value.clone(), value);
        path() => std::path::PathBuf::from("overridden");
    };
    assert_eq!(deps.copy(7), (7, 7));
    assert_eq!(deps.path(), std::path::PathBuf::from("overridden"));
}

#[cfg(test)]
mod conditional {
    use provider::conditional::Deps;
    renamed::stubs! {
        contract = provider::conditional::Deps;
        enabled => || 10;
        nested => || 20;
        unconditional => || 30;
    }
    #[test]
    fn exported_conditions_use_provider_features() {
        let deps = renamed::defs! { enabled() => 11; };
        assert_eq!((deps.enabled(), deps.nested(), deps.unconditional()), (11, 20, 30));
        let deps = renamed::defs! {
            contract = provider::conditional::Deps; target = provider::conditional::Deps;
            nested() => 21;
        };
        assert_eq!((deps.enabled(), deps.nested()), (1, 21));
    }
}
"###;

const INCOMPLETE_STUBS: &str = r###"
use traitstub::{deps, stubs};

deps! {
    first() -> u64, || 1;
    second() -> u64, || 2;
}
// Even unused definitions must fail cargo check when a stub is missing.
stubs! { first => || 0; }
"###;

const MALFORMED_DEPS: &str = r###"
traitstub::deps! { value() -> usize || 1; }
"###;

const MALFORMED_STUBS: &str = r###"
traitstub::deps! { value() -> usize, || 1; }
traitstub::stubs! { value -> || 0; }
"###;

const MALFORMED_DEFS: &str = r###"
traitstub::deps! { value() -> usize, || 1; }
traitstub::stubs! { value => || 0; }
fn broken() { let _deps = traitstub::defs! { value -> || 2; }; }
"###;

const EMPTY_STUBS: &str = r###"
traitstub::deps! { value() -> usize, || 1; }
traitstub::stubs! {}
"###;

const UNKNOWN_STUB: &str = r###"
traitstub::deps! { value() -> usize, || 1; }
traitstub::stubs! { missing => || 0; }
"###;

static NEXT_BUILD: AtomicUsize = AtomicUsize::new(0);

// This crate has no production dependencies or feature combinations. Compile its
// real entry point directly, then compile providers/consumers against that rlib.
// Rlib paths are explicit; -L is only for resolving transitive Rust dependencies.
struct Compiler {
    directory: PathBuf,
    library: PathBuf,
}

impl Compiler {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "traitstub-compilation-{}-{}",
            std::process::id(),
            NEXT_BUILD.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let library = directory.join("libtraitstub.rlib");
        let compiler = Self { directory, library };
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let output = compiler
            .command(&source, "traitstub")
            .args(["--crate-type", "rlib"])
            .arg("-o")
            .arg(&compiler.library)
            .output()
            .unwrap();
        assert_success(output);
        compiler
    }

    fn command(&self, source: &Path, name: &str) -> Command {
        let mut command = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        command
            .args(["--edition=2021", "--crate-name", name])
            .arg(source)
            .arg("-L")
            .arg(format!("dependency={}", self.directory.display()));
        command
    }

    fn source(&self, name: &str, source: &str) -> PathBuf {
        let path = self.directory.join(format!("{name}.rs"));
        std::fs::write(&path, source).unwrap();
        path
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[rstest]
#[case::missing_unused_stub(INCOMPLETE_STUBS, "stubs! is missing a stub for second")]
#[case::empty_stubs(EMPTY_STUBS, "stubs! is missing a stub for value")]
#[case::unknown_stub(UNKNOWN_STUB, "unknown dependency: missing")]
#[case::malformed_declaration(MALFORMED_DEPS, "deps! expects")]
#[case::malformed_shared_stub(MALFORMED_STUBS, "stubs! expects")]
#[case::malformed_override(MALFORMED_DEFS, "defs! expects")]
fn invalid_definitions_fail_even_when_unused(#[case] source: &str, #[case] expected: &str) {
    let compiler = Compiler::new();
    let source = compiler.source("rejected", source);
    let output = compiler
        .command(&source, "rejected")
        .args(["--crate-type", "lib", "--emit", "metadata", "--extern"])
        .arg(format!("traitstub={}", compiler.library.display()))
        .arg("-o")
        .arg(compiler.directory.join("rejected.rmeta"))
        .output()
        .unwrap();
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "invalid syntax unexpectedly compiled"
    );
    assert!(
        errors.contains(expected),
        "expected {expected:?}, got:\n{errors}"
    );
}

#[rstest]
#[case::ordinary_and_associated_types(
    DECLARATIVE_PROVIDER,
    DECLARATIVE_CONSUMER,
    "core_alias",
    false
)]
#[case::generics_and_provider_features(GENERIC_PROVIDER, GENERIC_CONSUMER, "renamed", false)]
#[case::different_consumer_features(GENERIC_PROVIDER, GENERIC_CONSUMER, "renamed", true)]
fn exported_contracts_work_between_independent_crates(
    #[case] provider: &str,
    #[case] consumer: &str,
    #[case] alias: &str,
    #[case] consumer_feature: bool,
) {
    let compiler = Compiler::new();
    let provider = compiler.source("provider", provider);
    let provider_library = compiler.directory.join("libprovider.rlib");
    let output = compiler
        .command(&provider, "provider")
        .args([
            "--crate-type",
            "rlib",
            "--cfg",
            "feature=\"present\"",
            "--extern",
        ])
        .arg(format!("{alias}={}", compiler.library.display()))
        .arg("-o")
        .arg(&provider_library)
        .output()
        .unwrap();
    assert_success(output);

    let consumer = compiler.source("consumer", consumer);
    let executable = compiler
        .directory
        .join(format!("consumer{}", std::env::consts::EXE_SUFFIX));
    let mut command = compiler.command(&consumer, "consumer");
    command
        .args(["--test", "--extern"])
        .arg(format!("{alias}={}", compiler.library.display()))
        .arg("--extern")
        .arg(format!("provider={}", provider_library.display()))
        .arg("-o")
        .arg(&executable);
    if consumer_feature {
        command.args(["--cfg", "feature=\"consumer_only\""]);
    }
    assert_success(command.output().unwrap());
    // Run the consumer's behavioral assertions, rather than only checking that
    // the exported macros compile. No assertions are disabled or moved to examples.
    assert_success(Command::new(executable).arg("--quiet").output().unwrap());
}
