use crate::support::sandbox::{sandbox, Sandbox};
use hamcrest2::assert_that;
use hamcrest2::prelude::*;
use serde_json::json;
use test_support::matchers::execs;

use volta_core::error::ExitCode;

fn platform_with_node_npm(node: &str, npm: &str) -> String {
    format!(
        r#"{{
  "node": {{
    "runtime": "{}",
    "npm": "{}"
  }},
  "pnpm": null,
  "yarn": null
}}"#,
        node, npm
    )
}

fn directory_platform_for(root: &std::path::Path, node: &str) -> String {
    directory_platform_json(
        root,
        json!({
            "node": node,
            "pnpm": null,
            "yarn": null,
        }),
    )
}

fn directory_platform_with_npm_for(root: &std::path::Path, node: &str, npm: &str) -> String {
    directory_platform_json(
        root,
        json!({
            "node": node,
            "npm": npm,
            "pnpm": null,
            "yarn": null,
        }),
    )
}

fn directory_platform_npm_only_for(root: &std::path::Path, npm: &str) -> String {
    directory_platform_json(
        root,
        json!({
            "node": null,
            "npm": npm,
            "pnpm": null,
            "yarn": null,
        }),
    )
}

fn directory_platform_json(root: &std::path::Path, platform: serde_json::Value) -> String {
    let mut platforms = serde_json::Map::new();
    platforms.insert(root.display().to_string(), platform);

    let mut contents = serde_json::Map::new();
    contents.insert(
        "platforms".to_string(),
        serde_json::Value::Object(platforms),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(contents))
        .expect("directory platform JSON should serialize")
}

fn empty_directory_platforms() -> &'static str {
    r#"{
  "platforms": {}
}"#
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        const NPM_SCRIPT: &str = "@echo off\r\n";
    } else {
        const NPM_SCRIPT: &str = "#!/bin/sh\n";
    }
}

#[test]
fn use_node_sets_directory_platform_without_changing_project() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .package_json(r#"{"name":"test-project"}"#)
        .setup_node_binary("9.27.6", "5.6.17", "")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("use node@9.27.6"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("[..]set node@9.27.6 for the current directory[..]")
    );

    assert_eq!(
        Sandbox::read_directory_platforms(),
        directory_platform_for(&s.root(), "9.27.6")
    );
    assert_eq!(s.read_package_json(), r#"{"name":"test-project"}"#);
}

#[test]
fn use_node_errors_when_version_is_missing_locally() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .build();

    assert_that!(
        s.volta("use node@9.27.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Could not find Node version matching `9.27.6` in the local inventory.[..]"
            )
            .with_stderr_contains("[..]Run `volta install node@9.27.6`[..]")
    );
}

#[test]
fn use_multiple_tools_applies_to_child_directories() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .project_file("child/.keep", "")
        .setup_node_binary("8.9.10", "4.5.6", "")
        .setup_node_binary("9.27.6", "5.6.17", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .env("VOLTA_LOGLEVEL", "debug")
        .build();

    assert_that!(
        s.volta("use node@9.27.6 npm@4.5.6"),
        execs().with_status(ExitCode::Success as i32)
    );
    assert_eq!(
        Sandbox::read_directory_platforms(),
        directory_platform_with_npm_for(&s.root(), "9.27.6", "4.5.6")
    );

    let mut npm = s.npm("--version");
    npm.cwd(s.root().join("child"));
    assert_that!(
        npm,
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stderr_contains("[..]Node: 9.27.6 from directory configuration")
            .with_stderr_contains("[..]npm: 4.5.6 from directory configuration")
    );
}

#[test]
fn use_list_reports_empty_directory_platforms() {
    let s = sandbox().build();

    assert_that!(
        s.volta("use list"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("No directory tool versions configured.")
    );
}

#[test]
fn use_list_shows_directory_platforms() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .setup_node_binary("9.27.6", "5.6.17", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .build();

    assert_that!(
        s.volta("use node@9.27.6 npm@4.5.6"),
        execs().with_status(ExitCode::Success as i32)
    );

    assert_that!(
        s.volta("use list"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains(format!("{} node@9.27.6 npm@4.5.6", s.root().display()))
    );
}

#[test]
fn unuse_removes_selected_tool_from_current_directory() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .setup_node_binary("9.27.6", "5.6.17", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .build();

    assert_that!(
        s.volta("use node@9.27.6 npm@4.5.6"),
        execs().with_status(ExitCode::Success as i32)
    );
    assert_that!(
        s.volta("unuse node"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("Removed selected tool versions for the current directory.")
    );

    assert_eq!(
        Sandbox::read_directory_platforms(),
        directory_platform_npm_only_for(&s.root(), "4.5.6")
    );
}

#[test]
fn unuse_all_clears_current_directory() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .setup_node_binary("9.27.6", "5.6.17", "")
        .build();

    assert_that!(
        s.volta("use node@9.27.6"),
        execs().with_status(ExitCode::Success as i32)
    );
    assert_that!(
        s.volta("unuse --all"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("Cleared tool versions for the current directory.")
    );

    assert_eq!(
        Sandbox::read_directory_platforms(),
        empty_directory_platforms()
    );
}

#[test]
fn unuse_dir_clears_specified_directory_without_cd() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .project_file("child/.keep", "")
        .setup_node_binary("9.27.6", "5.6.17", "")
        .build();

    let mut use_child = s.volta("use node@9.27.6");
    use_child.cwd(s.root().join("child"));
    assert_that!(use_child, execs().with_status(ExitCode::Success as i32));

    assert_eq!(
        Sandbox::read_directory_platforms(),
        directory_platform_for(&s.root().join("child"), "9.27.6")
    );

    assert_that!(
        s.volta("unuse --dir child --all"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("Cleared tool versions for child.")
    );

    assert_eq!(
        Sandbox::read_directory_platforms(),
        empty_directory_platforms()
    );
}
