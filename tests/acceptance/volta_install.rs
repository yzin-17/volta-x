use crate::support::sandbox::{
    sandbox, DistroMetadata, NpmFixture, PnpmFixture, Sandbox, Yarn1Fixture, YarnBerryFixture,
};
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

const YARN_1_VERSION_INFO: &str = r#"[
{"tag_name":"v1.2.42","assets":[{"name":"yarn-v1.2.42.tar.gz"}]},
{"tag_name":"v1.3.1","assets":[{"name":"yarn-v1.3.1.msi"}]},
{"tag_name":"v1.4.159","assets":[{"name":"yarn-v1.4.159.tar.gz"}]},
{"tag_name":"v1.7.71","assets":[{"name":"yarn-v1.7.71.tar.gz"}]},
{"tag_name":"v1.12.99","assets":[{"name":"yarn-v1.12.99.tar.gz"}]}
]"#;

const YARN_1_VERSION_FIXTURES: [DistroMetadata; 4] = [
    DistroMetadata {
        version: "1.12.99",
        compressed_size: 178,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "1.7.71",
        compressed_size: 176,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "1.4.159",
        compressed_size: 177,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "1.2.42",
        compressed_size: 174,
        uncompressed_size: Some(0x0028_0000),
    },
];

const YARN_BERRY_VERSION_INFO: &str = r#"{
    "name":"@yarnpkg/cli-dist",
    "dist-tags": { "latest":"3.12.99" },
    "versions": {
        "2.4.159": { "version":"2.4.159", "dist": { "shasum":"", "tarball":"" }},
        "3.2.42": { "version":"3.2.42", "dist": { "shasum":"", "tarball":"" }},
        "3.7.71": { "version":"3.7.71", "dist": { "shasum":"", "tarball":"" }},
        "3.12.99": { "version":"3.12.99", "dist": { "shasum":"", "tarball":"" }}
    }
}"#;

const YARN_BERRY_VERSION_FIXTURES: [DistroMetadata; 4] = [
    DistroMetadata {
        version: "2.4.159",
        compressed_size: 177,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "3.12.99",
        compressed_size: 178,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "3.7.71",
        compressed_size: 176,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "3.2.42",
        compressed_size: 174,
        uncompressed_size: Some(0x0028_0000),
    },
];

const PNPM_VERSION_INFO: &str = r#"
{
    "name":"pnpm",
    "dist-tags": { "latest":"7.7.1" },
    "versions": {
        "0.0.1": { "version":"0.0.1", "dist": { "shasum":"", "tarball":"" }},
        "6.34.0": { "version":"6.34.0", "dist": { "shasum":"", "tarball":"" }},
        "7.7.1": { "version":"7.7.1", "dist": { "shasum":"", "tarball":"" }}
    }
}
"#;

const PNPM_VERSION_FIXTURES: [DistroMetadata; 3] = [
    DistroMetadata {
        version: "0.0.1",
        compressed_size: 10,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "6.34.0",
        compressed_size: 500,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "7.7.1",
        compressed_size: 518,
        uncompressed_size: Some(0x0028_0000),
    },
];

const NPM_VERSION_INFO: &str = r#"
{
    "name":"npm",
    "dist-tags": { "latest":"8.1.5" },
    "versions": {
        "1.2.3": { "version":"1.2.3", "dist": { "shasum":"", "tarball":"" }},
        "4.5.6": { "version":"4.5.6", "dist": { "shasum":"", "tarball":"" }},
        "8.1.5": { "version":"8.1.5", "dist": { "shasum":"", "tarball":"" }}
    }
}
"#;

const NPM_VERSION_FIXTURES: [DistroMetadata; 3] = [
    DistroMetadata {
        version: "1.2.3",
        compressed_size: 239,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "4.5.6",
        compressed_size: 239,
        uncompressed_size: Some(0x0028_0000),
    },
    DistroMetadata {
        version: "8.1.5",
        compressed_size: 239,
        uncompressed_size: Some(0x0028_0000),
    },
];

#[test]
fn install_node_does_not_overwrite_existing_default() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "5.6.17"))
        .setup_node_binary("10.99.1040", "6.2.26", "")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("install node@10.99.1040"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains(
                "[..]node@10.99.1040 is installed; existing default version was not changed[..]"
            )
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node_npm("8.9.10", "5.6.17")
    );
}

#[test]
fn install_node_with_existing_default_hides_bundled_version() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "6.2.26"))
        .setup_node_binary("9.27.6", "5.6.17", "")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("install node@9.27.6"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_does_not_contain("[..](with npm@5.6.17)[..]")
    );
}

#[test]
fn install_npm_bundled_does_not_overwrite_existing_default() {
    let s = sandbox()
        .platform(&platform_with_node_npm("8.9.10", "6.2.26"))
        .node_npm_version_file("8.9.10", "5.6.7")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("install npm@bundled"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains(
                "[..]bundled npm is installed; existing default version was not changed[..]"
            )
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node_npm("8.9.10", "6.2.26")
    );
}

#[test]
fn install_npm_bundled_sets_default_when_no_custom_npm_exists() {
    let s = sandbox()
        .platform(&platform_with_node("8.9.10"))
        .node_npm_version_file("8.9.10", "5.6.7")
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("install npm@bundled"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("[..]set bundled npm (currently 5.6.7)[..]")
    );
}

#[test]
fn install_npm_sets_default_when_no_custom_npm_exists() {
    let s = sandbox()
        .platform(&platform_with_node("8.9.10"))
        .npm_available_versions(NPM_VERSION_INFO)
        .distro_mocks::<NpmFixture>(&NPM_VERSION_FIXTURES)
        .env("VOLTA_LOGLEVEL", "info")
        .build();

    assert_that!(
        s.volta("install npm@4.5.6"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("[..]installed and set npm@4.5.6 as default[..]")
    );

    assert_eq!(
        Sandbox::read_default_platform(),
        platform_with_node_npm("8.9.10", "4.5.6")
    );
}

#[test]
fn install_npm_without_node_errors() {
    let s = sandbox()
        .npm_available_versions(NPM_VERSION_INFO)
        .distro_mocks::<NpmFixture>(&NPM_VERSION_FIXTURES)
        .build();

    assert_that!(
        s.volta("install npm@4.5.6"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Cannot install npm because the default Node version is not set."
            )
    );
}

#[test]
fn install_pnpm_without_node_errors() {
    let s = sandbox()
        .pnpm_available_versions(PNPM_VERSION_INFO)
        .distro_mocks::<PnpmFixture>(&PNPM_VERSION_FIXTURES)
        .env("VOLTA_FEATURE_PNPM", "1")
        .build();

    assert_that!(
        s.volta("install pnpm@7.7.1"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Cannot install pnpm because the default Node version is not set."
            )
    );
}

#[test]
fn install_yarn_without_node_errors() {
    let s = sandbox()
        .yarn_1_available_versions(YARN_1_VERSION_INFO)
        .distro_mocks::<Yarn1Fixture>(&YARN_1_VERSION_FIXTURES)
        .build();

    assert_that!(
        s.volta("install yarn@1.2.42"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Cannot install Yarn because the default Node version is not set."
            )
    );
}

#[test]
fn install_yarn_3_without_node_errors() {
    let s = sandbox()
        .yarn_1_available_versions(YARN_1_VERSION_INFO)
        .yarn_berry_available_versions(YARN_BERRY_VERSION_INFO)
        .distro_mocks::<Yarn1Fixture>(&YARN_1_VERSION_FIXTURES)
        .distro_mocks::<YarnBerryFixture>(&YARN_BERRY_VERSION_FIXTURES)
        .build();

    assert_that!(
        s.volta("install yarn@3.2.42"),
        execs()
            .with_status(ExitCode::ConfigurationError as i32)
            .with_stderr_contains(
                "[..]Cannot install Yarn because the default Node version is not set."
            )
    );
}

#[test]
fn install_node_with_shadowed_binary() {
    #[cfg(windows)]
    const SCRIPT_FILENAME: &str = "node.bat";
    #[cfg(not(windows))]
    const SCRIPT_FILENAME: &str = "node";

    let s = sandbox()
        .setup_node_binary("10.99.1040", "6.2.26", "")
        .env("VOLTA_LOGLEVEL", "info")
        .prepend_exec_dir_to_path()
        .executable_file(SCRIPT_FILENAME, "echo hello world")
        .build();

    assert_that!(
        s.volta("install node@10.99.1040"),
        execs()
            .with_status(ExitCode::Success as i32)
            .with_stdout_contains("[..]is shadowed by another binary of the same name at [..]")
    );
}
