use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::helpers::{stdout, which};

#[test]
fn a_pretty_printed_answer_with_the_keys_in_another_order_is_read() {
    let output = root_package(
        "/work",
        "{\n  \"workspace_members\": [],\n  \"packages\": [\n    {\n      \"id\": \"path+file:///work#conveyor@0.0.0\",\n      \"manifest_path\": \"/work/Cargo.toml\",\n      \"version\": \"0.0.0\",\n      \"name\": \"conveyor\"\n    }\n  ]\n}\n",
    );

    assert_eq!(stdout(&output), "conveyor 0.0.0\n");
}

#[test]
fn an_answer_with_extra_whitespace_between_its_tokens_is_read() {
    let output = root_package(
        "/work",
        " { \"packages\" :\t[ { \"name\" : \"conveyor\" , \"version\" : \"0.4.0\" , \"manifest_path\" : \"/work/Cargo.toml\" } ] , \"workspace_members\" : [ ] }\n\n",
    );

    assert_eq!(stdout(&output), "conveyor 0.4.0\n");
}

#[test]
fn in_a_workspace_the_package_at_the_root_is_the_one_read() {
    let output = root_package(
        "/work",
        &packages(&[
            package("helper", "1.2.3", "/work/helper/Cargo.toml"),
            package("conveyor", "0.0.0", "/work/Cargo.toml"),
            package("other", "9.9.9", "/elsewhere/Cargo.toml"),
        ]),
    );

    assert_eq!(stdout(&output), "conveyor 0.0.0\n");
}

#[test]
fn a_root_whose_path_json_escapes_is_still_found() {
    let output = root_package(
        "/tmp/a \"quoted\" \\ dir",
        &packages(&[package(
            "conveyor",
            "0.0.0",
            "/tmp/a \\\"quoted\\\" \\\\ dir/Cargo.toml",
        )]),
    );

    assert_eq!(stdout(&output), "conveyor 0.0.0\n");
}

#[test]
fn a_name_spelled_with_json_escapes_is_read_as_cargo_meant_it() {
    let output = root_package(
        "/work",
        &packages(&[package("\\u0063onveyor", "0.0.0", "/work/Cargo.toml")]),
    );

    assert_eq!(stdout(&output), "conveyor 0.0.0\n");
}

#[test]
fn no_package_at_the_root_answers_nothing() {
    let output = root_package(
        "/work",
        &packages(&[package("helper", "1.2.3", "/work/helper/Cargo.toml")]),
    );

    assert_eq!(stdout(&output), "\n");
}

#[test]
fn a_root_with_a_path_that_only_starts_like_the_root_is_not_the_root() {
    let output = root_package(
        "/work",
        &packages(&[package("conveyor", "0.0.0", "/work/Cargo.toml.bak")]),
    );

    assert_eq!(stdout(&output), "\n");
}

#[test]
fn two_packages_at_the_root_are_unreadable() {
    let output = root_package(
        "/work",
        &packages(&[
            package("conveyor", "0.0.0", "/work/Cargo.toml"),
            package("conveyor", "1.0.0", "/work/Cargo.toml"),
        ]),
    );

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn a_root_package_whose_version_is_not_a_string_is_unreadable() {
    let output = root_package(
        "/work",
        "{\"packages\":[{\"name\":\"conveyor\",\"version\":null,\"manifest_path\":\"/work/Cargo.toml\"}]}",
    );

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn a_root_package_whose_name_is_not_a_string_is_unreadable() {
    let output = root_package(
        "/work",
        "{\"packages\":[{\"name\":[\"conveyor\"],\"version\":\"0.0.0\",\"manifest_path\":\"/work/Cargo.toml\"}]}",
    );

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn an_answer_that_is_not_json_is_unreadable() {
    let output = root_package("/work", "error: could not find `Cargo.toml`\n");

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn an_empty_answer_is_unreadable() {
    let output = root_package("/work", "");

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn two_answers_in_a_row_are_unreadable() {
    let one = packages(&[package("conveyor", "0.0.0", "/work/Cargo.toml")]);

    let output = root_package("/work", &format!("{one}\n{one}\n"));

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn an_answer_without_a_packages_array_is_unreadable() {
    let output = root_package("/work", "{\"workspace_members\":[]}");

    assert_eq!(stdout(&output), "unreadable\n");
}

#[test]
fn an_answer_whose_packages_are_an_object_is_unreadable() {
    let output = root_package(
        "/work",
        "{\"packages\":{\"root\":{\"name\":\"conveyor\",\"version\":\"0.0.0\",\"manifest_path\":\"/work/Cargo.toml\"}}}",
    );

    assert_eq!(stdout(&output), "unreadable\n");
}

fn root_package(root: &str, metadata: &str) -> Output {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/release-rewrite.sh");
    let mut child = Command::new(which("bash"))
        .arg("-c")
        .arg(". \"$1\" && root_package \"$2\"")
        .arg("bash")
        .arg(script)
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(metadata.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn packages(list: &[String]) -> String {
    format!(
        "{{\"packages\":[{}],\"workspace_members\":[]}}",
        list.join(",")
    )
}

fn package(name: &str, version: &str, manifest: &str) -> String {
    format!(
        "{{\"name\":\"{name}\",\"version\":\"{version}\",\"id\":\"path+file://{manifest}#{version}\",\"manifest_path\":\"{manifest}\"}}"
    )
}
