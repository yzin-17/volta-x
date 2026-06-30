use log::info;
use node_semver::Version;

use super::{resolve_local_version, Node, Npm, Pnpm, Yarn};
use crate::directory_platform::DirectoryPlatforms;
use crate::error::{ErrorKind, Fallible};
use crate::fs::{remove_dir_if_exists, remove_file_if_exists};
use crate::inventory::{node_versions, npm_versions, pnpm_versions, yarn_versions};
use crate::layout::volta_home;
use crate::platform::{Platform, PlatformSpec, Source};
use crate::session::Session;
use crate::style::{success_prefix, tool_version};
use crate::version::VersionSpec;

pub(super) enum CoreTool {
    Node,
    Npm,
    Pnpm,
    Yarn,
}

impl CoreTool {
    pub(super) fn uninstall(self, requested: VersionSpec, session: &mut Session) -> Fallible<()> {
        let version = self.resolve_local(requested)?;
        self.check_references(&version, session)?;
        self.remove(&version)?;

        info!(
            "{} uninstalled {}",
            success_prefix(),
            tool_version(self.name(), &version)
        );

        Ok(())
    }

    fn resolve_local(&self, requested: VersionSpec) -> Fallible<Version> {
        match self {
            CoreTool::Node => resolve_local_version("Node", requested, node_versions()?),
            CoreTool::Npm => resolve_local_version("npm", requested, npm_versions()?),
            CoreTool::Pnpm => resolve_local_version("pnpm", requested, pnpm_versions()?),
            CoreTool::Yarn => resolve_local_version("Yarn", requested, yarn_versions()?),
        }
    }

    fn check_references(&self, version: &Version, session: &mut Session) -> Fallible<()> {
        if let Some(platform) = session.default_platform()? {
            if self.matches_platform_spec(version, platform) {
                return self.in_use_error(version, "the default platform");
            }
        }

        if DirectoryPlatforms::current()?.any_references_tool_version(self.key(), version) {
            return self.in_use_error(version, "a directory platform configured by `volta use`");
        }

        if let Some(platform) = Platform::current(session)? {
            if self.matches_platform(version, &platform)
                && self.source_for_platform(&platform) != Some(Source::Default)
            {
                return self.in_use_error(version, "the current project platform");
            }
        }

        Ok(())
    }

    fn remove(&self, version: &Version) -> Fallible<()> {
        let home = volta_home()?;
        let version_string = version.to_string();

        match self {
            CoreTool::Node => {
                remove_dir_if_exists(home.node_image_dir(&version_string))?;
                remove_file_if_exists(
                    home.node_inventory_dir()
                        .join(Node::archive_filename(version)),
                )?;
                remove_file_if_exists(home.node_npm_version_file(&version_string))
            }
            CoreTool::Npm => {
                remove_dir_if_exists(home.npm_image_dir(&version_string))?;
                remove_file_if_exists(
                    home.npm_inventory_dir()
                        .join(Npm::archive_filename(&version_string)),
                )
            }
            CoreTool::Pnpm => {
                remove_dir_if_exists(home.pnpm_image_dir(&version_string))?;
                remove_file_if_exists(
                    home.pnpm_inventory_dir()
                        .join(Pnpm::archive_filename(&version_string)),
                )
            }
            CoreTool::Yarn => {
                remove_dir_if_exists(home.yarn_image_dir(&version_string))?;
                remove_file_if_exists(
                    home.yarn_inventory_dir()
                        .join(Yarn::archive_filename(&version_string)),
                )
            }
        }
    }

    fn matches_platform_spec(&self, version: &Version, platform: &PlatformSpec) -> bool {
        match self {
            CoreTool::Node => &platform.node == version,
            CoreTool::Npm => platform.npm.as_ref() == Some(version),
            CoreTool::Pnpm => platform.pnpm.as_ref() == Some(version),
            CoreTool::Yarn => platform.yarn.as_ref() == Some(version),
        }
    }

    fn matches_platform(&self, version: &Version, platform: &Platform) -> bool {
        match self {
            CoreTool::Node => &platform.node.value == version,
            CoreTool::Npm => platform.npm.as_ref().map(|npm| &npm.value) == Some(version),
            CoreTool::Pnpm => platform.pnpm.as_ref().map(|pnpm| &pnpm.value) == Some(version),
            CoreTool::Yarn => platform.yarn.as_ref().map(|yarn| &yarn.value) == Some(version),
        }
    }

    fn source_for_platform(&self, platform: &Platform) -> Option<Source> {
        match self {
            CoreTool::Node => Some(platform.node.source),
            CoreTool::Npm => platform.npm.as_ref().map(|npm| npm.source),
            CoreTool::Pnpm => platform.pnpm.as_ref().map(|pnpm| pnpm.source),
            CoreTool::Yarn => platform.yarn.as_ref().map(|yarn| yarn.source),
        }
    }

    fn in_use_error<T>(&self, version: &Version, source: &str) -> Fallible<T> {
        Err(ErrorKind::ToolVersionInUse {
            tool: self.name().into(),
            version: version.to_string(),
            source: source.into(),
        }
        .into())
    }

    fn name(&self) -> &'static str {
        match self {
            CoreTool::Node => "node",
            CoreTool::Npm => "npm",
            CoreTool::Pnpm => "pnpm",
            CoreTool::Yarn => "yarn",
        }
    }

    fn key(&self) -> &'static str {
        self.name()
    }
}
