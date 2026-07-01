use std::collections::BTreeMap;
use std::fs::write;
use std::path::{Path, PathBuf};

use crate::error::{Context, ErrorKind, Fallible, VoltaError};
use crate::fs::read_file;
use crate::layout::volta_home;
use crate::platform::{Platform, Sourced};
use crate::version::{option_version_serde, parse_version};
use dunce::canonicalize;
use log::debug;
use node_semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
pub struct DirectoryPlatformSpec {
    pub node: Option<Version>,
    pub npm: Option<Option<Version>>,
    pub pnpm: Option<Version>,
    pub yarn: Option<Version>,
}

impl DirectoryPlatformSpec {
    pub fn is_empty(&self) -> bool {
        self.node.is_none() && self.npm.is_none() && self.pnpm.is_none() && self.yarn.is_none()
    }

    pub fn merge(self, base: Option<Platform>) -> Option<Platform> {
        match (self.node, base) {
            (Some(node), base) => Some(Platform {
                node: Sourced::with_directory(node),
                npm: self
                    .npm
                    .map(|npm| npm.map(Sourced::with_directory))
                    .unwrap_or_else(|| base.as_ref().and_then(|platform| platform.npm.clone())),
                pnpm: self
                    .pnpm
                    .map(Sourced::with_directory)
                    .or_else(|| base.as_ref().and_then(|platform| platform.pnpm.clone())),
                yarn: self
                    .yarn
                    .map(Sourced::with_directory)
                    .or_else(|| base.as_ref().and_then(|platform| platform.yarn.clone())),
            }),
            (None, Some(mut platform)) => {
                if let Some(npm) = self.npm {
                    platform.npm = npm.map(Sourced::with_directory);
                }
                if let Some(pnpm) = self.pnpm {
                    platform.pnpm = Some(Sourced::with_directory(pnpm));
                }
                if let Some(yarn) = self.yarn {
                    platform.yarn = Some(Sourced::with_directory(yarn));
                }
                Some(platform)
            }
            (None, None) => None,
        }
    }
}

pub struct DirectoryPlatforms {
    platforms: BTreeMap<PathBuf, DirectoryPlatformSpec>,
}

impl DirectoryPlatforms {
    pub fn current() -> Fallible<Self> {
        let path = volta_home()?.directory_platforms_file();
        let src = read_file(path).with_context(|| ErrorKind::ReadPlatformError {
            file: path.to_owned(),
        })?;

        let platforms = src
            .map(SerialDirectoryPlatforms::try_from)
            .transpose()?
            .map(TryInto::try_into)
            .transpose()?
            .unwrap_or_default();

        Ok(DirectoryPlatforms { platforms })
    }

    pub fn find_for(&self, directory: &Path) -> Option<DirectoryPlatformSpec> {
        self.platforms
            .iter()
            .filter(|(configured, _)| directory.starts_with(configured))
            .max_by_key(|(configured, _)| configured.components().count())
            .map(|(_, platform)| platform.clone())
    }

    pub fn entries(&self) -> impl Iterator<Item = (&Path, &DirectoryPlatformSpec)> {
        self.platforms
            .iter()
            .map(|(path, platform)| (path.as_path(), platform))
    }

    pub fn any_references_tool_version(&self, tool: &str, version: &Version) -> bool {
        self.platforms.values().any(|platform| match tool {
            "node" => platform.node.as_ref() == Some(version),
            "npm" => platform.npm.as_ref().and_then(Option::as_ref) == Some(version),
            "pnpm" => platform.pnpm.as_ref() == Some(version),
            "yarn" => platform.yarn.as_ref() == Some(version),
            _ => false,
        })
    }

    pub fn set_current_dir<F>(&mut self, update: F) -> Fallible<()>
    where
        F: FnOnce(&mut DirectoryPlatformSpec),
    {
        let directory =
            canonicalize(std::env::current_dir().with_context(|| ErrorKind::CurrentDirError)?)
                .with_context(|| ErrorKind::CurrentDirError)?;
        let platform = self.platforms.entry(directory.clone()).or_default();
        update(platform);
        debug!("Set directory platform for '{}'", directory.display());
        self.save()
    }

    pub fn unset_current_dir<F>(&mut self, update: F) -> Fallible<()>
    where
        F: FnOnce(&mut DirectoryPlatformSpec),
    {
        let directory =
            canonicalize(std::env::current_dir().with_context(|| ErrorKind::CurrentDirError)?)
                .with_context(|| ErrorKind::CurrentDirError)?;
        self.unset_dir(&directory, update)
    }

    pub fn unset_dir<F>(&mut self, directory: &Path, update: F) -> Fallible<()>
    where
        F: FnOnce(&mut DirectoryPlatformSpec),
    {
        let directory = canonicalize(directory).with_context(|| ErrorKind::CurrentDirError)?;
        let should_remove = if let Some(platform) = self.platforms.get_mut(&directory) {
            update(platform);
            platform.is_empty()
        } else {
            false
        };

        if should_remove {
            self.platforms.remove(&directory);
        }

        debug!("Unset directory platform for '{}'", directory.display());
        self.save()
    }

    pub fn clear_current_dir(&mut self) -> Fallible<()> {
        let directory =
            canonicalize(std::env::current_dir().with_context(|| ErrorKind::CurrentDirError)?)
                .with_context(|| ErrorKind::CurrentDirError)?;
        self.clear_dir(&directory)
    }

    pub fn clear_dir(&mut self, directory: &Path) -> Fallible<()> {
        let directory = canonicalize(directory).with_context(|| ErrorKind::CurrentDirError)?;
        self.platforms.remove(&directory);
        debug!("Cleared directory platform for '{}'", directory.display());
        self.save()
    }

    fn save(&self) -> Fallible<()> {
        let path = volta_home()?.directory_platforms_file();
        let src = SerialDirectoryPlatforms::of(self).into_json()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| ErrorKind::CreateDirError {
                dir: parent.to_owned(),
            })?;
        }
        write(path, src).with_context(|| ErrorKind::WritePlatformError {
            file: path.to_owned(),
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
struct SerialDirectoryPlatforms {
    #[serde(default)]
    platforms: BTreeMap<PathBuf, SerialDirectoryPlatform>,
}

impl SerialDirectoryPlatforms {
    fn of(source: &DirectoryPlatforms) -> Self {
        SerialDirectoryPlatforms {
            platforms: source
                .platforms
                .iter()
                .map(|(path, platform)| (path.clone(), SerialDirectoryPlatform::of(platform)))
                .collect(),
        }
    }

    fn into_json(self) -> Fallible<String> {
        serde_json::to_string_pretty(&self).with_context(|| ErrorKind::StringifyPlatformError)
    }
}

impl TryFrom<String> for SerialDirectoryPlatforms {
    type Error = VoltaError;

    fn try_from(src: String) -> Fallible<Self> {
        let result = if src.is_empty() {
            serde_json::de::from_str("{}")
        } else {
            serde_json::de::from_str(&src)
        };

        result.with_context(|| ErrorKind::ParsePlatformError)
    }
}

impl TryFrom<SerialDirectoryPlatforms> for BTreeMap<PathBuf, DirectoryPlatformSpec> {
    type Error = VoltaError;

    fn try_from(value: SerialDirectoryPlatforms) -> Fallible<Self> {
        value
            .platforms
            .into_iter()
            .map(|(path, platform)| platform.try_into().map(|platform| (path, platform)))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct SerialDirectoryPlatform {
    #[serde(default)]
    #[serde(with = "option_version_serde")]
    node: Option<Version>,
    #[serde(
        default,
        skip_serializing_if = "is_inherited_npm",
        serialize_with = "npm_version_serde::serialize",
        deserialize_with = "npm_version_serde::deserialize"
    )]
    npm: Option<Option<String>>,
    #[serde(default)]
    #[serde(with = "option_version_serde")]
    pnpm: Option<Version>,
    #[serde(default)]
    #[serde(with = "option_version_serde")]
    yarn: Option<Version>,
}

fn is_inherited_npm(npm: &Option<Option<String>>) -> bool {
    npm.is_none()
}

mod npm_version_serde {
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(npm: &Option<Option<String>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match npm {
            None => serializer.serialize_unit(),
            Some(None) => serializer.serialize_none(),
            Some(Some(version)) => serializer.serialize_some(version),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NpmVersionVisitor;

        impl<'de> Visitor<'de> for NpmVersionVisitor {
            type Value = Option<Option<String>>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a version string or null")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Some(None))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Some(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer).map(|version| Some(Some(version)))
            }
        }

        deserializer.deserialize_option(NpmVersionVisitor)
    }
}

impl SerialDirectoryPlatform {
    fn of(source: &DirectoryPlatformSpec) -> Self {
        SerialDirectoryPlatform {
            node: source.node.clone(),
            npm: source
                .npm
                .as_ref()
                .map(|npm| npm.as_ref().map(Version::to_string)),
            pnpm: source.pnpm.clone(),
            yarn: source.yarn.clone(),
        }
    }
}

impl TryFrom<SerialDirectoryPlatform> for DirectoryPlatformSpec {
    type Error = VoltaError;

    fn try_from(value: SerialDirectoryPlatform) -> Fallible<Self> {
        Ok(DirectoryPlatformSpec {
            node: value.node,
            npm: value
                .npm
                .map(|npm| npm.map(parse_version).transpose())
                .transpose()?,
            pnpm: value.pnpm,
            yarn: value.yarn,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DirectoryPlatformSpec, DirectoryPlatforms, SerialDirectoryPlatforms};
    use node_semver::Version;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn version(version: &str) -> Version {
        Version::parse(version).expect("valid version")
    }

    #[test]
    fn round_trips_directory_platforms_json() {
        let directory = PathBuf::from("/repo/app");
        let platforms = DirectoryPlatforms {
            platforms: BTreeMap::from([(
                directory.clone(),
                DirectoryPlatformSpec {
                    node: Some(version("18.19.0")),
                    npm: Some(Some(version("10.2.3"))),
                    pnpm: Some(version("8.15.1")),
                    yarn: None,
                },
            )]),
        };

        let json = SerialDirectoryPlatforms::of(&platforms)
            .into_json()
            .expect("serializes");
        let parsed = SerialDirectoryPlatforms::try_from(json).expect("parses");
        let parsed: BTreeMap<PathBuf, DirectoryPlatformSpec> = parsed.try_into().expect("converts");

        assert_eq!(parsed.get(&directory), platforms.platforms.get(&directory));
    }

    #[test]
    fn preserves_bundled_npm_marker() {
        let directory = PathBuf::from("/repo/app");
        let platforms = DirectoryPlatforms {
            platforms: BTreeMap::from([(
                directory.clone(),
                DirectoryPlatformSpec {
                    node: Some(version("18.19.0")),
                    npm: Some(None),
                    pnpm: None,
                    yarn: None,
                },
            )]),
        };

        let json = SerialDirectoryPlatforms::of(&platforms)
            .into_json()
            .expect("serializes");
        let parsed = SerialDirectoryPlatforms::try_from(json).expect("parses");
        let parsed: BTreeMap<PathBuf, DirectoryPlatformSpec> = parsed.try_into().expect("converts");

        assert_eq!(parsed.get(&directory), platforms.platforms.get(&directory));
    }

    #[test]
    fn finds_longest_matching_ancestor() {
        let root = PathBuf::from("/repo");
        let app = PathBuf::from("/repo/app");
        let platforms = DirectoryPlatforms {
            platforms: BTreeMap::from([
                (
                    root,
                    DirectoryPlatformSpec {
                        node: Some(version("16.20.2")),
                        ..DirectoryPlatformSpec::default()
                    },
                ),
                (
                    app,
                    DirectoryPlatformSpec {
                        node: Some(version("18.19.0")),
                        ..DirectoryPlatformSpec::default()
                    },
                ),
            ]),
        };

        let platform = platforms
            .find_for(&PathBuf::from("/repo/app/packages/widget"))
            .expect("matched");

        assert_eq!(platform.node, Some(version("18.19.0")));
    }
}
