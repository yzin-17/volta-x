use std::env;
use std::fmt;

use crate::directory_platform::DirectoryPlatforms;
use crate::error::{ErrorKind, Fallible};
use crate::node_version_file;
use crate::session::Session;
use crate::tool::{Node, Npm, Pnpm, Yarn};
use crate::VOLTA_FEATURE_PNPM;
use node_semver::Version;

mod image;
mod system;
// Note: The tests get their own module because we need them to run as a single unit to prevent
// clobbering environment variable changes
#[cfg(test)]
mod tests;

pub use image::Image;
pub use system::System;

/// The source with which a version is associated
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Source {
    /// Represents a version from the user default platform
    Default,

    /// Represents a version from a project manifest
    Project,

    /// Represents a version from a directory platform configured by `volta use`
    Directory,

    /// Represents a version from a .nvmrc file
    Nvmrc,

    /// Represents a version from a .node-version file
    NodeVersion,

    /// Represents a version from a pinned Binary platform
    Binary,

    /// Represents a version from the command line (via `volta run`)
    CommandLine,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Default => write!(f, "default"),
            Source::Project => write!(f, "project"),
            Source::Directory => write!(f, "directory"),
            Source::Nvmrc => write!(f, "nvmrc"),
            Source::NodeVersion => write!(f, "node-version"),
            Source::Binary => write!(f, "binary"),
            Source::CommandLine => write!(f, "command-line"),
        }
    }
}

pub struct Sourced<T> {
    pub value: T,
    pub source: Source,
}

impl<T> Sourced<T> {
    pub fn with_default(value: T) -> Self {
        Sourced {
            value,
            source: Source::Default,
        }
    }

    pub fn with_project(value: T) -> Self {
        Sourced {
            value,
            source: Source::Project,
        }
    }

    pub fn with_directory(value: T) -> Self {
        Sourced {
            value,
            source: Source::Directory,
        }
    }

    pub fn with_source(value: T, source: Source) -> Self {
        Sourced { value, source }
    }

    pub fn with_binary(value: T) -> Self {
        Sourced {
            value,
            source: Source::Binary,
        }
    }

    pub fn with_command_line(value: T) -> Self {
        Sourced {
            value,
            source: Source::CommandLine,
        }
    }
}

impl<T> Sourced<T> {
    pub fn as_ref(&self) -> Sourced<&T> {
        Sourced {
            value: &self.value,
            source: self.source,
        }
    }
}

impl<T> Sourced<&T>
where
    T: Clone,
{
    pub fn cloned(self) -> Sourced<T> {
        Sourced {
            value: self.value.clone(),
            source: self.source,
        }
    }
}

impl<T> Clone for Sourced<T>
where
    T: Clone,
{
    fn clone(&self) -> Sourced<T> {
        Sourced {
            value: self.value.clone(),
            source: self.source,
        }
    }
}

/// Represents 3 possible states: Having a value, not having a value, and inheriting a value
#[cfg_attr(test, derive(Eq, PartialEq, Debug))]
#[derive(Clone, Default)]
pub enum InheritOption<T> {
    Some(T),
    None,
    #[default]
    Inherit,
}

impl<T> InheritOption<T> {
    /// Applies a function to the contained value (if any)
    pub fn map<U, F>(self, f: F) -> InheritOption<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            InheritOption::Some(value) => InheritOption::Some(f(value)),
            InheritOption::None => InheritOption::None,
            InheritOption::Inherit => InheritOption::Inherit,
        }
    }

    /// Converts the `InheritOption` into a regular `Option` by inheriting from the provided value if needed
    pub fn inherit(self, other: Option<T>) -> Option<T> {
        match self {
            InheritOption::Some(value) => Some(value),
            InheritOption::None => None,
            InheritOption::Inherit => other,
        }
    }
}

impl<T> From<InheritOption<T>> for Option<T> {
    fn from(base: InheritOption<T>) -> Option<T> {
        base.inherit(None)
    }
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
/// Represents the specification of a single Platform, regardless of the source
pub struct PlatformSpec {
    pub node: Version,
    pub npm: Option<Version>,
    pub pnpm: Option<Version>,
    pub yarn: Option<Version>,
}

impl PlatformSpec {
    /// Convert this PlatformSpec into a Platform with all sources set to `Default`
    pub fn as_default(&self) -> Platform {
        Platform {
            node: Sourced::with_default(self.node.clone()),
            npm: self.npm.clone().map(Sourced::with_default),
            pnpm: self.pnpm.clone().map(Sourced::with_default),
            yarn: self.yarn.clone().map(Sourced::with_default),
        }
    }

    /// Convert this PlatformSpec into a Platform with all sources set to `Project`
    pub fn as_project(&self) -> Platform {
        Platform {
            node: Sourced::with_project(self.node.clone()),
            npm: self.npm.clone().map(Sourced::with_project),
            pnpm: self.pnpm.clone().map(Sourced::with_project),
            yarn: self.yarn.clone().map(Sourced::with_project),
        }
    }

    /// Convert this PlatformSpec into a Platform with all sources set to `Binary`
    pub fn as_binary(&self) -> Platform {
        Platform {
            node: Sourced::with_binary(self.node.clone()),
            npm: self.npm.clone().map(Sourced::with_binary),
            pnpm: self.pnpm.clone().map(Sourced::with_binary),
            yarn: self.yarn.clone().map(Sourced::with_binary),
        }
    }
}

/// Represents a (maybe) platform with values from the command line
#[derive(Clone)]
pub struct CliPlatform {
    pub node: Option<Version>,
    pub npm: InheritOption<Version>,
    pub pnpm: InheritOption<Version>,
    pub yarn: InheritOption<Version>,
}

impl CliPlatform {
    /// Merges the `CliPlatform` with a `Platform`, inheriting from the base where needed
    pub fn merge(self, base: Platform) -> Platform {
        Platform {
            node: self.node.map_or(base.node, Sourced::with_command_line),
            npm: self.npm.map(Sourced::with_command_line).inherit(base.npm),
            pnpm: self.pnpm.map(Sourced::with_command_line).inherit(base.pnpm),
            yarn: self.yarn.map(Sourced::with_command_line).inherit(base.yarn),
        }
    }
}

impl From<CliPlatform> for Option<Platform> {
    /// Converts the `CliPlatform` into a possible Platform without a base from which to inherit
    fn from(base: CliPlatform) -> Option<Platform> {
        match base.node {
            None => None,
            Some(node) => Some(Platform {
                node: Sourced::with_command_line(node),
                npm: base.npm.map(Sourced::with_command_line).into(),
                pnpm: base.pnpm.map(Sourced::with_command_line).into(),
                yarn: base.yarn.map(Sourced::with_command_line).into(),
            }),
        }
    }
}

/// Represents a real Platform, with Versions pulled from one or more `PlatformSpec`s
#[derive(Clone)]
pub struct Platform {
    pub node: Sourced<Version>,
    pub npm: Option<Sourced<Version>>,
    pub pnpm: Option<Sourced<Version>>,
    pub yarn: Option<Sourced<Version>>,
}

impl Platform {
    /// Returns the user's currently active platform, if any
    ///
    /// Active platform is determined by layering configuration sources from
    /// lowest to highest priority:
    ///
    /// - user default platform
    /// - `.node-version`
    /// - `.nvmrc`
    /// - project platform configured by `volta pin`
    /// - directory platform configured by `volta use`
    ///
    /// Higher-priority Node-only sources inherit npm, pnpm, and Yarn from
    /// lower-priority sources when available.
    pub fn current(session: &mut Session) -> Fallible<Option<Self>> {
        let default_platform = session.default_platform()?.map(PlatformSpec::as_default);
        let directory =
            dunce::canonicalize(std::env::current_dir().map_err(|_| build_path_error())?)
                .map_err(|_| build_path_error())?;

        let node_version_platform = merge_node(
            node_version_file::node_version_platform(&directory)?,
            default_platform,
        );
        let nvmrc_platform = merge_node(
            node_version_file::nvmrc_platform(&directory)?,
            node_version_platform,
        );

        let project_platform = if let Some(project_platform) = session.project_platform()? {
            Some(merge_platform_spec(
                project_platform,
                Source::Project,
                nvmrc_platform,
            ))
        } else {
            nvmrc_platform
        };

        Ok(DirectoryPlatforms::current()?
            .find_for(&directory)
            .map(|platform| platform.merge(project_platform.clone()))
            .unwrap_or(project_platform))
    }

    /// Check out a `Platform` into a fully-realized `Image`
    ///
    /// This will ensure that all necessary tools are fetched and available for execution
    pub fn checkout(self, session: &mut Session) -> Fallible<Image> {
        Node::new(self.node.value.clone()).ensure_fetched(session)?;

        if let Some(Sourced { value: version, .. }) = &self.npm {
            Npm::new(version.clone()).ensure_fetched(session)?;
        }

        // Only force download of the pnpm version if the pnpm feature flag is set. If it isn't,
        // then we won't be using the `Pnpm` tool to execute (we will be relying on the global
        // package logic), so fetching the Pnpm version would only be redundant work.
        if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
            if let Some(Sourced { value: version, .. }) = &self.pnpm {
                Pnpm::new(version.clone()).ensure_fetched(session)?;
            }
        }

        if let Some(Sourced { value: version, .. }) = &self.yarn {
            Yarn::new(version.clone()).ensure_fetched(session)?;
        }

        Ok(Image {
            node: self.node,
            npm: self.npm,
            pnpm: self.pnpm,
            yarn: self.yarn,
        })
    }
}

fn merge_node(node: Option<Sourced<Version>>, base: Option<Platform>) -> Option<Platform> {
    match (node, base) {
        (Some(node), base) => Some(Platform {
            node,
            npm: base.as_ref().and_then(|platform| platform.npm.clone()),
            pnpm: base.as_ref().and_then(|platform| platform.pnpm.clone()),
            yarn: base.as_ref().and_then(|platform| platform.yarn.clone()),
        }),
        (None, base) => base,
    }
}

fn merge_platform_spec(spec: &PlatformSpec, source: Source, base: Option<Platform>) -> Platform {
    Platform {
        node: Sourced::with_source(spec.node.clone(), source),
        npm: spec
            .npm
            .clone()
            .map(|npm| Sourced::with_source(npm, source))
            .or_else(|| base.as_ref().and_then(|platform| platform.npm.clone())),
        pnpm: spec
            .pnpm
            .clone()
            .map(|pnpm| Sourced::with_source(pnpm, source))
            .or_else(|| base.as_ref().and_then(|platform| platform.pnpm.clone())),
        yarn: spec
            .yarn
            .clone()
            .map(|yarn| Sourced::with_source(yarn, source))
            .or_else(|| base.as_ref().and_then(|platform| platform.yarn.clone())),
    }
}

fn build_path_error() -> ErrorKind {
    ErrorKind::BuildPathError
}

#[cfg(test)]
mod merge_tests {
    use super::{merge_node, merge_platform_spec, Platform, PlatformSpec, Source, Sourced};
    use node_semver::Version;

    fn version(version: &str) -> Version {
        Version::parse(version).expect("valid version")
    }

    #[test]
    fn node_only_sources_inherit_package_managers_from_base() {
        let base = Platform {
            node: Sourced::with_default(version("8.9.10")),
            npm: Some(Sourced::with_default(version("4.5.6"))),
            pnpm: Some(Sourced::with_default(version("6.34.0"))),
            yarn: Some(Sourced::with_default(version("1.7.71"))),
        };

        let merged = merge_node(
            Some(Sourced::with_source(version("10.99.1040"), Source::Nvmrc)),
            Some(base),
        )
        .expect("merged platform");

        assert_eq!(merged.node.value, version("10.99.1040"));
        assert_eq!(merged.node.source, Source::Nvmrc);
        assert_eq!(merged.npm.expect("npm").value, version("4.5.6"));
        assert_eq!(merged.pnpm.expect("pnpm").value, version("6.34.0"));
        assert_eq!(merged.yarn.expect("yarn").value, version("1.7.71"));
    }

    #[test]
    fn project_platform_without_npm_inherits_npm_from_base() {
        let spec = PlatformSpec {
            node: version("6.19.62"),
            npm: None,
            pnpm: None,
            yarn: None,
        };
        let base = Platform {
            node: Sourced::with_default(version("8.9.10")),
            npm: Some(Sourced::with_default(version("4.5.6"))),
            pnpm: None,
            yarn: None,
        };

        let merged = merge_platform_spec(&spec, Source::Project, Some(base));

        assert_eq!(merged.node.value, version("6.19.62"));
        assert_eq!(merged.node.source, Source::Project);
        assert_eq!(merged.npm.expect("npm").value, version("4.5.6"));
    }
}
