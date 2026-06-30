use crate::support::sandbox::{sandbox, Sandbox};
use hamcrest2::assert_that;
use hamcrest2::prelude::*;
use test_support::matchers::execs;

use volta_core::error::ExitCode;

fn platform_with_node(node: &str) -> String {
    format!(
        r#"{{
  "node": {{
    "runtime": "{}",
    "npm": null
  }},
  "pnpm": null,
  "yarn": null
}}"#,
        node
    )
}

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

#[test]
fn default_node_sets_installed_node_as_default() {
    let s = sandbox()
        .platform(&platform_with_node("8.9.10"))
        .setup_node_binary("9.27.6", "5.6.17", "")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("default node@9.27.6"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("[..]set node@9.27.6 as default[..]")
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node("9.27.6")
    );
}

#[test]
fn default_node_errors_when_version_is_missing_locally() {
    let s = sandbox().platform(&platform_with_node("8.9.10")).build();

    assert_that!(
        s.volta("default node@9.27.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Could not find Node version matching `9.27.6` in the local inventory.[..]"
            )
            .with_stderr_contains("[..]Run `volta install node@9.27.6`[..]")
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node("8.9.10")
    );
}

#[test]
fn default_npm_sets_installed_npm_as_default() {
    let s = sandbox()
        .platform(&platform_with_node("8.9.10"))
        .setup_npm_binary("4.5.6", "")
        .build();

    assert_that!(
        s.volta("default npm@4.5.6"),
        execs().with_status(ExitCode::Success as i32)
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node_npm("8.9.10", "4.5.6")
    );
}

#[test]
fn default_npm_bundled_clears_custom_npm() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "4.5.6"))
        .node_npm_version_file("8.9.10", "5.6.7")
        .build();

    assert_that!(
        s.volta("default npm@bundled"),
        execs().with_status(ExitCode::Success as i32)
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node("8.9.10")
    );
}
