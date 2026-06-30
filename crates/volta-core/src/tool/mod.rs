use std::env;
use std::fmt::{self, Display};
use std::path::PathBuf;

use crate::directory_platform::DirectoryPlatforms;
use crate::error::{ErrorKind, Fallible};
use crate::inventory::{node_versions, npm_versions, pnpm_versions, yarn_versions};
use crate::layout::volta_home;
use crate::session::Session;
use crate::style::{note_prefix, success_prefix, tool_version};
use crate::sync::VoltaLock;
use crate::version::{VersionSpec, VersionTag};
use crate::VOLTA_FEATURE_PNPM;
use cfg_if::cfg_if;
use log::{debug, info};
use node_semver::{Range, Version};

pub mod node;
pub mod npm;
pub mod package;
pub mod pnpm;
mod registry;
mod serial;
mod uninstall;
pub mod yarn;

pub use node::{
    load_default_npm_version, Node, NODE_DISTRO_ARCH, NODE_DISTRO_EXTENSION, NODE_DISTRO_OS,
};
pub use npm::{BundledNpm, Npm};
pub use package::{BinConfig, Package, PackageConfig, PackageManifest};
pub use pnpm::Pnpm;
pub use registry::PackageDetails;
pub use yarn::Yarn;

fn debug_already_fetched<T: Display>(tool: T) {
    debug!("{} has already been fetched, skipping download", tool);
}

fn info_installed<T: Display>(tool: T) {
    info!("{} installed and set {tool} as default", success_prefix());
}

fn info_defaulted<T: Display>(tool: T) {
    info!("{} set {tool} as default", success_prefix());
}

fn info_default_preserved<T: Display>(tool: T) {
    info!(
        "{} {tool} is installed; existing default version was not changed",
        note_prefix()
    );
}

fn info_fetched<T: Display>(tool: T) {
    info!("{} fetched {tool}", success_prefix());
}

fn info_pinned<T: Display>(tool: T) {
    info!("{} pinned {tool} in package.json", success_prefix());
}

fn info_directory_used<T: Display>(tool: T) {
    info!("{} set {tool} for the current directory", success_prefix());
}

fn info_project_version<P, D>(project_version: P, default_version: D)
where
    P: Display,
    D: Display,
{
    info!(
        r#"{} you are using {project_version} in the current project; to
         instead use {default_version}, run `volta pin {default_version}`"#,
        note_prefix()
    );
}

/// Trait representing all of the actions that can be taken with a tool
pub trait Tool: Display {
    /// Fetch a Tool into the local inventory
    fn fetch(self: Box<Self>, session: &mut Session) -> Fallible<()>;
    /// Install a tool, making it the default so it is available everywhere on the user's machine
    fn install(self: Box<Self>, session: &mut Session) -> Fallible<()>;
    /// Pin a tool in the local project so that it is usable within the project
    fn pin(self: Box<Self>, session: &mut Session) -> Fallible<()>;
}

/// Specification for a tool and its associated version.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Spec {
    Node(VersionSpec),
    Npm(VersionSpec),
    Pnpm(VersionSpec),
    Yarn(VersionSpec),
    Package(String, VersionSpec),
}

impl Spec {
    /// Resolve a tool spec into a fully realized Tool that can be fetched
    pub fn resolve(self, session: &mut Session) -> Fallible<Box<dyn Tool>> {
        match self {
            Spec::Node(version) => {
                let version = node::resolve(version, session)?;
                Ok(Box::new(Node::new(version)))
            }
            Spec::Npm(version) => match npm::resolve(version, session)? {
                Some(version) => Ok(Box::new(Npm::new(version))),
                None => Ok(Box::new(BundledNpm)),
            },
            Spec::Pnpm(version) => {
                // If the pnpm feature flag is set, use the special-cased package manager logic
                // to handle resolving (and ultimately fetching / installing) pnpm. If not, then
                // fall back to the global package behavior, which was the case prior to pnpm
                // support being added
                if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
                    let version = pnpm::resolve(version, session)?;
                    Ok(Box::new(Pnpm::new(version)))
                } else {
                    let package = Package::new("pnpm".to_owned(), version)?;
                    Ok(Box::new(package))
                }
            }
            Spec::Yarn(version) => {
                let version = yarn::resolve(version, session)?;
                Ok(Box::new(Yarn::new(version)))
            }
            // When using global package install, we allow the package manager to perform the version resolution
            Spec::Package(name, version) => {
                let package = Package::new(name, version)?;
                Ok(Box::new(package))
            }
        }
    }

    /// Install a tool, fetching it if needed. Core runtime tools only become the default
    /// when no default for that tool exists yet; package installs keep their existing behavior.
    pub fn install(self, session: &mut Session) -> Fallible<()> {
        match self {
            Spec::Node(version) => {
                let version = node::resolve(version, session)?;
                let node = Node::new(version.clone());

                if session.default_platform()?.is_some() {
                    node.ensure_fetched(session)?;
                    info_default_preserved(node);
                    Ok(())
                } else {
                    Box::new(node).install(session)
                }
            }
            Spec::Npm(version)
                if !is_bundled_npm(&version) && session.default_platform()?.is_none() =>
            {
                Err(ErrorKind::NoDefaultNodeVersion { tool: "npm".into() }.into())
            }
            Spec::Npm(version) => match npm::resolve(version, session)? {
                Some(version) => {
                    let npm = Npm::new(version.clone());

                    if session.default_platform()?.is_some() {
                        npm.ensure_fetched(session)?;
                        info_default_preserved(npm);
                        Ok(())
                    } else {
                        Box::new(npm).install(session)
                    }
                }
                None => {
                    if session
                        .default_platform()?
                        .is_some_and(|platform| platform.npm.is_some())
                    {
                        info_default_preserved("bundled npm");
                        Ok(())
                    } else {
                        Box::new(BundledNpm).install(session)
                    }
                }
            },
            Spec::Pnpm(version) => {
                if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
                    if session.default_platform()?.is_none() {
                        return Err(ErrorKind::NoDefaultNodeVersion {
                            tool: "pnpm".into(),
                        }
                        .into());
                    }

                    let version = pnpm::resolve(version, session)?;
                    let pnpm = Pnpm::new(version.clone());

                    if session
                        .default_platform()?
                        .is_some_and(|platform| platform.pnpm.is_some())
                    {
                        pnpm.ensure_fetched(session)?;
                        info_default_preserved(pnpm);
                        Ok(())
                    } else {
                        Box::new(pnpm).install(session)
                    }
                } else {
                    Box::new(Package::new("pnpm".to_owned(), version)?).install(session)
                }
            }
            Spec::Yarn(version) => {
                if session.default_platform()?.is_none() {
                    return Err(ErrorKind::NoDefaultNodeVersion {
                        tool: "Yarn".into(),
                    }
                    .into());
                }

                let version = yarn::resolve(version, session)?;
                let yarn = Yarn::new(version.clone());

                if session
                    .default_platform()?
                    .is_some_and(|platform| platform.yarn.is_some())
                {
                    yarn.ensure_fetched(session)?;
                    info_default_preserved(yarn);
                    Ok(())
                } else {
                    Box::new(yarn).install(session)
                }
            }
            Spec::Package(name, version) => Box::new(Package::new(name, version)?).install(session),
        }
    }

    /// Set the default version of an already-fetched core tool without downloading.
    pub fn default(self, session: &mut Session) -> Fallible<()> {
        let _lock = VoltaLock::acquire();

        match self {
            Spec::Node(version) => {
                let version = resolve_local_version("Node", version, node_versions()?)?;
                session.toolchain_mut()?.set_active_node(&version)?;
                info_defaulted(Node::new(version));
                check_shim_reachable("node");
                Ok(())
            }
            Spec::Npm(version) => match resolve_local_npm(version)? {
                Some(version) => {
                    session
                        .toolchain_mut()?
                        .set_active_npm(Some(version.clone()))?;
                    info_defaulted(Npm::new(version));
                    check_shim_reachable("npm");
                    Ok(())
                }
                None => Box::new(BundledNpm).install(session),
            },
            Spec::Pnpm(version) => {
                if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
                    let version = resolve_local_version("pnpm", version, pnpm_versions()?)?;
                    session
                        .toolchain_mut()?
                        .set_active_pnpm(Some(version.clone()))?;
                    info_defaulted(Pnpm::new(version));
                    check_shim_reachable("pnpm");
                    Ok(())
                } else {
                    Err(ErrorKind::CannotDefaultPackage {
                        package: "pnpm".into(),
                    }
                    .into())
                }
            }
            Spec::Yarn(version) => {
                let version = resolve_local_version("Yarn", version, yarn_versions()?)?;
                session
                    .toolchain_mut()?
                    .set_active_yarn(Some(version.clone()))?;
                info_defaulted(Yarn::new(version));
                check_shim_reachable("yarn");
                Ok(())
            }
            Spec::Package(name, _) => Err(ErrorKind::CannotDefaultPackage { package: name }.into()),
        }
    }

    /// Set the version of an already-fetched core tool for the current directory without downloading.
    pub fn use_current_dir(self, _session: &mut Session) -> Fallible<()> {
        let _lock = VoltaLock::acquire();
        let mut directory_platforms = DirectoryPlatforms::current()?;

        match self {
            Spec::Node(version) => {
                let version = resolve_local_version("Node", version, node_versions()?)?;
                directory_platforms.set_current_dir(|platform| {
                    platform.node = Some(version.clone());
                })?;
                info_directory_used(Node::new(version));
                Ok(())
            }
            Spec::Npm(version) => match resolve_local_npm(version)? {
                Some(version) => {
                    directory_platforms.set_current_dir(|platform| {
                        platform.npm = Some(Some(version.clone()));
                    })?;
                    info_directory_used(Npm::new(version));
                    Ok(())
                }
                None => {
                    directory_platforms.set_current_dir(|platform| {
                        platform.npm = Some(None);
                    })?;
                    info_directory_used("bundled npm");
                    Ok(())
                }
            },
            Spec::Pnpm(version) => {
                if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
                    let version = resolve_local_version("pnpm", version, pnpm_versions()?)?;
                    directory_platforms.set_current_dir(|platform| {
                        platform.pnpm = Some(version.clone());
                    })?;
                    info_directory_used(Pnpm::new(version));
                    Ok(())
                } else {
                    Err(ErrorKind::CannotUsePackage {
                        package: "pnpm".into(),
                    }
                    .into())
                }
            }
            Spec::Yarn(version) => {
                let version = resolve_local_version("Yarn", version, yarn_versions()?)?;
                directory_platforms.set_current_dir(|platform| {
                    platform.yarn = Some(version.clone());
                })?;
                info_directory_used(Yarn::new(version));
                Ok(())
            }
            Spec::Package(name, _) => Err(ErrorKind::CannotUsePackage { package: name }.into()),
        }
    }

    /// Uninstall a tool, removing it from the local inventory
    ///
    /// This is implemented on Spec, instead of Resolved, because there is currently no need to
    /// resolve the specific version before uninstalling a tool.
    pub fn uninstall(self, session: &mut Session) -> Fallible<()> {
        match self {
            Spec::Node(VersionSpec::None) => Err(ErrorKind::Unimplemented {
                feature: "Uninstalling node".into(),
            }
            .into()),
            Spec::Node(version) => uninstall::CoreTool::Node.uninstall(version, session),
            Spec::Npm(VersionSpec::None) => Err(ErrorKind::Unimplemented {
                feature: "Uninstalling npm".into(),
            }
            .into()),
            Spec::Npm(version) => uninstall::CoreTool::Npm.uninstall(version, session),
            Spec::Pnpm(version) => {
                if env::var_os(VOLTA_FEATURE_PNPM).is_some() {
                    match version {
                        VersionSpec::None => Err(ErrorKind::Unimplemented {
                            feature: "Uninstalling pnpm".into(),
                        }
                        .into()),
                        version => uninstall::CoreTool::Pnpm.uninstall(version, session),
                    }
                } else {
                    package::uninstall("pnpm")
                }
            }
            Spec::Yarn(VersionSpec::None) => Err(ErrorKind::Unimplemented {
                feature: "Uninstalling yarn".into(),
            }
            .into()),
            Spec::Yarn(version) => uninstall::CoreTool::Yarn.uninstall(version, session),
            Spec::Package(name, _) => package::uninstall(&name),
        }
    }

    /// The name of the tool, without the version, used for messaging
    pub fn name(&self) -> &str {
        match self {
            Spec::Node(_) => "Node",
            Spec::Npm(_) => "npm",
            Spec::Pnpm(_) => "pnpm",
            Spec::Yarn(_) => "Yarn",
            Spec::Package(name, _) => name,
        }
    }
}

fn is_bundled_npm(version: &VersionSpec) -> bool {
    matches!(version, VersionSpec::Tag(VersionTag::Custom(tag)) if tag == "bundled")
}

fn resolve_local_npm(version: VersionSpec) -> Fallible<Option<Version>> {
    match version {
        VersionSpec::Tag(VersionTag::Custom(tag)) if tag == "bundled" => Ok(None),
        version => resolve_local_version("npm", version, npm_versions()?).map(Some),
    }
}

pub fn resolve_local_node(version: VersionSpec) -> Fallible<Version> {
    resolve_local_version("Node", version, node_versions()?)
}

pub(crate) fn resolve_local_version(
    tool: &str,
    matching: VersionSpec,
    versions: std::collections::BTreeSet<Version>,
) -> Fallible<Version> {
    let display = matching.to_string();
    let version = match matching {
        VersionSpec::Exact(version) => versions.get(&version).cloned(),
        VersionSpec::Semver(range) => newest_matching(versions, &range),
        VersionSpec::None | VersionSpec::Tag(VersionTag::Latest) => {
            versions.into_iter().next_back()
        }
        VersionSpec::Tag(VersionTag::Lts) if tool == "Node" => versions.into_iter().next_back(),
        VersionSpec::Tag(tag) => {
            return Err(ErrorKind::ToolVersionNotInstalled {
                tool: tool.into(),
                matching: tag.to_string(),
            }
            .into())
        }
    };

    version.ok_or_else(|| {
        ErrorKind::ToolVersionNotInstalled {
            tool: tool.into(),
            matching: display,
        }
        .into()
    })
}

fn newest_matching(
    versions: std::collections::BTreeSet<Version>,
    range: &Range,
) -> Option<Version> {
    versions
        .into_iter()
        .rev()
        .find(|version| range.satisfies(version))
}

impl Display for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Spec::Node(ref version) => tool_version("node", version),
            Spec::Npm(ref version) => tool_version("npm", version),
            Spec::Pnpm(ref version) => tool_version("pnpm", version),
            Spec::Yarn(ref version) => tool_version("yarn", version),
            Spec::Package(ref name, ref version) => tool_version(name, version),
        };
        f.write_str(&s)
    }
}

/// Represents the result of checking if a tool is available locally or not
///
/// If a fetch is required, will include an exclusive lock on the Volta directory where possible
enum FetchStatus {
    AlreadyFetched,
    FetchNeeded(Option<VoltaLock>),
}

/// Uses the supplied `already_fetched` predicate to determine if a tool is available or not.
///
/// This uses double-checking logic, to correctly handle concurrent fetch requests:
///
/// - If `already_fetched` indicates that a fetch is needed, we acquire an exclusive lock on the Volta directory
/// - Then, we check _again_, to confirm that no other process completed the fetch while we waited for the lock
///
/// Note: If acquiring the lock fails, we proceed anyway, since the fetch is still necessary.
fn check_fetched<F>(already_fetched: F) -> Fallible<FetchStatus>
where
    F: Fn() -> Fallible<bool>,
{
    if !already_fetched()? {
        let lock = match VoltaLock::acquire() {
            Ok(l) => Some(l),
            Err(_) => {
                debug!("Unable to acquire lock on Volta directory!");
                None
            }
        };

        if !already_fetched()? {
            Ok(FetchStatus::FetchNeeded(lock))
        } else {
            Ok(FetchStatus::AlreadyFetched)
        }
    } else {
        Ok(FetchStatus::AlreadyFetched)
    }
}

fn download_tool_error(tool: Spec, from_url: impl AsRef<str>) -> impl FnOnce() -> ErrorKind {
    let from_url = from_url.as_ref().to_string();
    || ErrorKind::DownloadToolNetworkError { tool, from_url }
}

fn registry_fetch_error(
    tool: impl AsRef<str>,
    from_url: impl AsRef<str>,
) -> impl FnOnce() -> ErrorKind {
    let tool = tool.as_ref().to_string();
    let from_url = from_url.as_ref().to_string();
    || ErrorKind::RegistryFetchError { tool, from_url }
}

cfg_if!(
    if #[cfg(windows)] {
        const PATH_VAR_NAME: &str = "Path";
    } else {
        const PATH_VAR_NAME: &str = "PATH";
    }
);

/// Check if a newly-installed shim is first on the PATH. If it isn't, we want to inform the user
/// that they'll want to move it to the start of PATH to make sure things work as expected.
pub fn check_shim_reachable(shim_name: &str) {
    let Some(expected_dir) = find_expected_shim_dir(shim_name) else {
        return;
    };

    let Ok(resolved) = which::which(shim_name) else {
        info!(
            "{} cannot find command {}. Please ensure that {} is available on your {}.",
            note_prefix(),
            shim_name,
            expected_dir.display(),
            PATH_VAR_NAME,
        );
        return;
    };

    if !resolved.starts_with(&expected_dir) {
        info!(
            "{} {} is shadowed by another binary of the same name at {}. To ensure your commands work as expected, please move {} to the start of your {}.",
            note_prefix(),
            shim_name,
            resolved.display(),
            expected_dir.display(),
            PATH_VAR_NAME
        );
    }
}

/// Locate the base directory for the relevant shim in the Volta directories.
///
/// On Unix, all of the shims, including the default ones, are installed in `VoltaHome::shim_dir`
#[cfg(unix)]
fn find_expected_shim_dir(_shim_name: &str) -> Option<PathBuf> {
    volta_home().ok().map(|home| home.shim_dir().to_owned())
}

/// Locate the base directory for the relevant shim in the Volta directories.
///
/// On Windows, the default shims (node, npm, yarn, etc.) are installed in `Program Files`
/// alongside the Volta binaries. To determine where we should be checking, we first look for the
/// relevant shim inside of `VoltaHome::shim_dir`. If it's there, we use that directory. If it
/// isn't, we assume it must be a default shim and return `VoltaInstall::root`, which is where
/// Volta itself is installed.
#[cfg(windows)]
fn find_expected_shim_dir(shim_name: &str) -> Option<PathBuf> {
    use crate::layout::volta_install;

    let home = volta_home().ok()?;

    if home.shim_file(shim_name).exists() {
        Some(home.shim_dir().to_owned())
    } else {
        volta_install()
            .ok()
            .map(|install| install.root().to_owned())
    }
}
