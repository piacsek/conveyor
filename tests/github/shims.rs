use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, MutexGuard};

use conveyor::sources::github::CliGh;

pub static SHIMS: Mutex<()> = Mutex::new(());

pub fn serialized() -> MutexGuard<'static, ()> {
    SHIMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn shim(dir: &tempfile::TempDir, script: &str) -> CliGh {
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    CliGh::with_program(gh)
}

pub const RECORD_PGID: &str = "ps -o pgid= -p $$ > \"$0.pgid\"";

pub fn own_pgid() -> String {
    let output = std::process::Command::new("ps")
        .args(["-o", "pgid=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

pub fn recorded_pgid(dir: &tempfile::TempDir) -> String {
    fs::read_to_string(dir.path().join("bin/gh.pgid"))
        .unwrap()
        .trim()
        .to_string()
}
