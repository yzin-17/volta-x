//! Tests for `volta uninstall`.

use crate::support::sandbox::{sandbox, Sandbox};
use hamcrest2::assert_that;
use hamcrest2::prelude::*;
use test_support::matchers::execs;
use volta_core::error::ExitCode;

const PKG_CONFIG_BASIC: &str = r#"{
  "name": "cowsay",
  "version": "1.4.0",
  "platform": {
    "node": "11.10.1",
    "npm": "6.7.0",
    "yarn": null
  },
  "bins": [
    "cowsay",
    "cowthink"
  ],
  "manager": "Npm"
}"#;

const PKG_CONFIG_NO_BINS: &str = r#"{
  "name": "cowsay",
  "version": "1.4.0",
  "platform": {
    "node": "11.10.1",
    "npm": "6.7.0",
    "yarn": null
  },
  "bins": [],
  "manager": "Npm"
}"#;

fn bin_config(name: &str) -> String {
    format!(
        r#"{{
  "name": "{}",
  "package": "cowsay",
  "version": "1.4.0",
  "platform": {{
    "node": "11.10.1",
    "npm": "6.7.0",
    "yarn": null
  }},
  "manager": "Npm"
}}"#,
        name
    )
}

const VOLTA_LOGLEVEL: &str = "VOLTA_LOGLEVEL";

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

fn package_json_with_pinned_node(node: &str) -> String {
    format!(
        r#"{{
  "name": "pinned",
  "volta": {{
    "node": "{}"
  }}
}}"#,
        node
    )
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        const SCRIPT: &str = "@echo off\r\n";
    } else {
        const SCRIPT: &str = "#!/bin/sh\n";
    }
}

#[test]
fn uninstall_nonexistent_pkg() {
    // if the package doesn't exist, it should just inform the user but not throw an error
    let s = sandbox().env(VOLTA_LOGLEVEL, "info").build();

    assert_that!(
        s.volta("uninstall cowsay"),
        execs()
            .with_status(0)
            .with_stderr_contains("[..]No package 'cowsay' found to uninstall")
    );
}

#[test]
fn uninstall_package_basic() {
    // basic uninstall - everything exists, and everything except the cached
    // inventory files should be deleted
    let s = sandbox()
        .package_config("cowsay", PKG_CONFIG_BASIC)
        .binary_config("cowsay", &bin_config("cowsay"))
        .binary_config("cowthink", &bin_config("cowthink"))
        .shim("cowsay")
        .shim("cowthink")
        .package_image("cowsay", "1.4.0", None)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall cowsay"),
        execs()
            .with_status(0)
            .with_stdout_contains("Removed executable 'cowsay' installed by 'cowsay'")
            .with_stdout_contains("Removed executable 'cowthink' installed by 'cowsay'")
            .with_stdout_contains("[..]package 'cowsay' uninstalled")
    );

    // check that everything is deleted
    assert!(!Sandbox::package_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowthink"));
    assert!(!Sandbox::shim_exists("cowsay"));
    assert!(!Sandbox::shim_exists("cowthink"));
    assert!(!Sandbox::package_image_exists("cowsay"));
}

// The setup here is the same as the above, but here we check to make sure that
// if the user supplies a version, we error correctly.
#[test]
fn uninstall_package_basic_with_version() {
    // basic uninstall - everything exists, and everything except the cached
    // inventory files should be deleted
    let s = sandbox()
        .package_config("cowsay", PKG_CONFIG_BASIC)
        .binary_config("cowsay", &bin_config("cowsay"))
        .binary_config("cowthink", &bin_config("cowthink"))
        .shim("cowsay")
        .shim("cowthink")
        .package_image("cowsay", "1.4.0", None)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall cowsay@1.4.0"),
        execs().with_status(1).with_stderr_contains(
            "[..]error: uninstalling specific versions of tools is not supported yet."
        )
    );
}

#[test]
fn uninstall_specific_node_version() {
    let s = sandbox()
        .setup_node_binary("9.27.6", "5.6.17", SCRIPT)
        .setup_node_binary("10.99.1040", "6.4.1", SCRIPT)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall node@9"),
        execs()
            .with_status(0)
            .with_stdout_contains("[..]uninstalled node@9.27.6")
    );

    assert!(!Sandbox::node_image_exists("9.27.6"));
    assert!(!Sandbox::node_npm_version_file_exists("9.27.6"));
    assert!(Sandbox::node_image_exists("10.99.1040"));
}

#[test]
fn uninstall_specific_npm_version() {
    let s = sandbox()
        .setup_npm_binary("4.5.6", SCRIPT)
        .setup_npm_binary("6.2.26", SCRIPT)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall npm@4"),
        execs()
            .with_status(0)
            .with_stdout_contains("[..]uninstalled npm@4.5.6")
    );

    assert!(!Sandbox::npm_image_exists("4.5.6"));
    assert!(Sandbox::npm_image_exists("6.2.26"));
}

#[test]
fn uninstall_specific_pnpm_version() {
    let s = sandbox()
        .setup_pnpm_binary("7.7.1", SCRIPT)
        .setup_pnpm_binary("8.15.1", SCRIPT)
        .env("VOLTA_FEATURE_PNPM", "1")
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall pnpm@7"),
        execs()
            .with_status(0)
            .with_stdout_contains("[..]uninstalled pnpm@7.7.1")
    );

    assert!(!Sandbox::pnpm_image_exists("7.7.1"));
    assert!(Sandbox::pnpm_image_exists("8.15.1"));
}

#[test]
fn uninstall_specific_yarn_version() {
    let s = sandbox()
        .setup_yarn_binary("1.12.99", SCRIPT)
        .setup_yarn_binary("1.22.22", SCRIPT)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall yarn@1.12"),
        execs()
            .with_status(0)
            .with_stdout_contains("[..]uninstalled yarn@1.12.99")
    );

    assert!(!Sandbox::yarn_image_exists("1.12.99"));
    assert!(Sandbox::yarn_image_exists("1.22.22"));
}

#[test]
fn uninstall_specific_node_refuses_default_reference() {
    let s = sandbox()
        .platform(&platform_with_node_npm("9.27.6", "5.6.17"))
        .setup_node_binary("9.27.6", "5.6.17", SCRIPT)
        .build();

    assert_that!(
        s.volta("uninstall node@9.27.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains("[..]Cannot uninstall node@9.27.6 because node@9.27.6 is still referenced by the default platform.")
    );

    assert!(Sandbox::node_image_exists("9.27.6"));
}

#[test]
fn uninstall_specific_node_refuses_current_project_reference() {
    let s = sandbox()
        .package_json(&package_json_with_pinned_node("9.27.6"))
        .setup_node_binary("9.27.6", "5.6.17", SCRIPT)
        .build();

    assert_that!(
        s.volta("uninstall node@9.27.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains("[..]Cannot uninstall node@9.27.6 because node@9.27.6 is still referenced by the current project platform.")
    );

    assert!(Sandbox::node_image_exists("9.27.6"));
}

#[test]
fn uninstall_specific_node_refuses_directory_platform_reference() {
    let s = sandbox()
        .setup_node_binary("9.27.6", "5.6.17", SCRIPT)
        .build();

    assert_that!(s.volta("use node@9.27.6"), execs().with_status(0));

    assert_that!(
        s.volta("uninstall node@9.27.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains("[..]Cannot uninstall node@9.27.6 because node@9.27.6 is still referenced by a directory platform configured by `volta use`.")
    );

    assert!(Sandbox::node_image_exists("9.27.6"));
}

#[test]
fn uninstall_package_no_bins() {
    // the package doesn't contain any executables, it should uninstall without error
    // (normally installing a package with no executables should not happen)
    let s = sandbox()
        .package_config("cowsay", PKG_CONFIG_NO_BINS)
        .package_image("cowsay", "1.4.0", None)
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall cowsay"),
        execs()
            .with_status(0)
            .with_stdout_contains("[..]package 'cowsay' uninstalled")
    );

    // check that everything is deleted
    assert!(!Sandbox::package_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowthink"));
    assert!(!Sandbox::shim_exists("cowsay"));
    assert!(!Sandbox::shim_exists("cowthink"));
    assert!(!Sandbox::package_image_exists("cowsay"));
}

#[test]
fn uninstall_package_no_image() {
    // there is no unpacked & initialized package, but everything should be removed
    // (without erroring and failing to remove everything)
    let s = sandbox()
        .package_config("cowsay", PKG_CONFIG_BASIC)
        .binary_config("cowsay", &bin_config("cowsay"))
        .binary_config("cowthink", &bin_config("cowthink"))
        .shim("cowsay")
        .shim("cowthink")
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall cowsay"),
        execs()
            .with_status(0)
            .with_stdout_contains("Removed executable 'cowsay' installed by 'cowsay'")
            .with_stdout_contains("Removed executable 'cowthink' installed by 'cowsay'")
            .with_stdout_contains("[..]package 'cowsay' uninstalled")
    );

    // check that everything is deleted
    assert!(!Sandbox::package_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowthink"));
    assert!(!Sandbox::shim_exists("cowsay"));
    assert!(!Sandbox::shim_exists("cowthink"));
    assert!(!Sandbox::package_image_exists("cowsay"));
}

#[test]
fn uninstall_package_orphaned_bins() {
    // the package config does not exist, but for some reason there are orphaned binaries
    // those should be removed
    let s = sandbox()
        .binary_config("cowsay", &bin_config("cowsay"))
        .binary_config("cowthink", &bin_config("cowthink"))
        .shim("cowsay")
        .shim("cowthink")
        .env(VOLTA_LOGLEVEL, "info")
        .build();

    assert_that!(
        s.volta("uninstall cowsay"),
        execs()
            .with_status(0)
            .with_stdout_contains("Removed executable 'cowsay' installed by 'cowsay'")
            .with_stdout_contains("Removed executable 'cowthink' installed by 'cowsay'")
            .with_stdout_contains("[..]package 'cowsay' uninstalled")
    );

    // check that everything is deleted
    assert!(!Sandbox::bin_config_exists("cowsay"));
    assert!(!Sandbox::bin_config_exists("cowthink"));
    assert!(!Sandbox::shim_exists("cowsay"));
    assert!(!Sandbox::shim_exists("cowthink"));
}

#[test]
fn uninstall_runtime() {
    let s = sandbox().build();
    assert_that!(
        s.volta("uninstall node"),
        execs()
            .with_status(1)
            .with_stderr_contains("[..]error: Uninstalling node is not supported yet.")
    )
}
