#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod defs;
mod test;

fn main() -> ExitCode {
    let mut args: Vec<_> = std::env::args_os().collect();
    let mut bless = false;
    if args.get(1).is_some_and(|arg| arg == "--bless") {
        bless = true;
        args.remove(1);
    }

    let args = libtest_mimic::Arguments::from_iter(args);

    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap();
    let tests_path = Path::new("ui-tests");

    let tests_paths = gather_tests(&root_path, tests_path);

    let cmd_bin_path = Path::new(env!("CARGO_BIN_EXE_rsjsonnet"))
        .canonicalize()
        .unwrap();

    let mut tests = Vec::new();
    for test_subpath in tests_paths {
        let root_path = root_path.clone();
        let test_name = test_subpath.display().to_string();
        let cmd_bin_path = cmd_bin_path.clone();
        tests.push(libtest_mimic::Trial::test(test_name, move || {
            test::run(&root_path, &test_subpath, &cmd_bin_path, bless).map_err(Into::into)
        }));
    }

    let conclusion = libtest_mimic::run(&args, tests);
    if conclusion.has_failed() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn gather_tests(root_path: &Path, tests_path: &Path) -> BTreeSet<PathBuf> {
    let mut tests = BTreeSet::new();

    let mut dir_queue = Vec::new();
    dir_queue.push(tests_path.to_path_buf());

    while let Some(current_sub_path) = dir_queue.pop() {
        let current_dir = root_path.join(&current_sub_path);
        for entry in current_dir.read_dir().unwrap() {
            let entry = entry.unwrap();
            let entry_type = entry.file_type().unwrap();
            let entry_name = entry.file_name();

            if entry_type.is_dir() {
                dir_queue.push(current_sub_path.join(entry_name));
                continue;
            }

            let extension = Path::new(&entry_name).extension();
            if extension == Some(OsStr::new("jsonnet")) {
                let inserted = tests.insert(current_sub_path.join(entry_name));
                assert!(inserted);
            }
        }
    }

    tests
}
