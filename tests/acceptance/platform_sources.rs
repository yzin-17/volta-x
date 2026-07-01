use crate::support::sandbox::sandbox;
use hamcrest2::assert_that;
use hamcrest2::prelude::*;
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

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        const NPM_SCRIPT: &str = "@echo off\r\n";
    } else {
        const NPM_SCRIPT: &str = "#!/bin/sh\n";
    }
}

#[test]
fn use_wins_over_nvmrc_node_version_and_default() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .project_file(".node-version", "6\n")
        .project_file("child/.nvmrc", "v10.99.1040\n")
        .layout_file("v4")
        .setup_node_binary("6.19.62", "3.10.10", "")
        .setup_node_binary("8.9.10", "4.5.6", "")
        .setup_node_binary("9.27.6", "5.6.17", "")
        .setup_node_binary("10.99.1040", "6.4.1", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .env("VOLTA_LOGLEVEL", "debug")
        .build();

    assert_that!(
        s.volta("use node@9.27.6"),
        execs().with_status(ExitCode::Success as i32)
    );

    let mut npm = s.npm("--version");
    npm.cwd(s.root().join("child"));
    assert_that!(
        npm,
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stderr_contains("[..]Node: 9.27.6 from directory configuration")
            .with_stderr_contains("[..]npm: 4.5.6 from default configuration")
    );
}

#[test]
fn nvmrc_wins_over_node_version_and_default() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .project_file(".node-version", "6\n")
        .project_file("child/.nvmrc", "v10.99.1040\n")
        .layout_file("v4")
        .setup_node_binary("6.19.62", "3.10.10", "")
        .setup_node_binary("8.9.10", "4.5.6", "")
        .setup_node_binary("10.99.1040", "6.4.1", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .env("VOLTA_LOGLEVEL", "debug")
        .build();

    let mut npm = s.npm("--version");
    npm.cwd(s.root().join("child"));
    assert_that!(
        npm,
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stderr_contains("[..]Node: 10.99.1040 from nvmrc configuration")
            .with_stderr_contains("[..]npm: 4.5.6 from default configuration")
    );
}

#[test]
fn use_wins_over_project_pin() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .package_json(
            r#"{
  "name": "pinned",
  "volta": {
    "node": "6.19.62"
  }
}"#,
        )
        .project_file(".nvmrc", "10.99.1040\n")
        .layout_file("v4")
        .setup_node_binary("6.19.62", "3.10.10", "")
        .setup_node_binary("8.9.10", "4.5.6", "")
        .setup_node_binary("9.27.6", "5.6.17", "")
        .setup_node_binary("10.99.1040", "6.4.1", "")
        .setup_npm_binary("4.5.6", NPM_SCRIPT)
        .env("VOLTA_LOGLEVEL", "debug")
        .build();

    assert_that!(
        s.volta("use node@9.27.6"),
        execs().with_status(ExitCode::Success as i32)
    );

    assert_that!(
        s.npm("--version"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stderr_contains("[..]Node: 9.27.6 from directory configuration")
            .with_stderr_contains("[..]npm: 4.5.6 from default configuration")
    );
}
