use std::path::{Path, PathBuf};

use crate::error::{Context, ErrorKind, Fallible};
use crate::fs::read_file;
use crate::platform::{Source, Sourced};
use crate::tool::resolve_local_node;
use crate::version::VersionSpec;
use node_semver::Version;

pub fn nvmrc_platform(directory: &Path) -> Fallible<Option<Sourced<Version>>> {
    file_platform(directory, ".nvmrc", Source::Nvmrc)
}

pub fn node_version_platform(directory: &Path) -> Fallible<Option<Sourced<Version>>> {
    file_platform(directory, ".node-version", Source::NodeVersion)
}

fn file_platform(
    directory: &Path,
    filename: &str,
    source: Source,
) -> Fallible<Option<Sourced<Version>>> {
    let Some(file) = find_ancestor_file(directory, filename) else {
        return Ok(None);
    };

    let src = read_file(&file)
        .with_context(|| ErrorKind::ReadPlatformError {
            file: file.to_owned(),
        })?
        .unwrap_or_default();
    let version_spec = parse_node_version_file(&src)?;
    let version = resolve_local_node(version_spec)?;

    Ok(Some(Sourced {
        value: version,
        source,
    }))
}

fn find_ancestor_file(directory: &Path, filename: &str) -> Option<PathBuf> {
    let mut current = directory.to_owned();

    loop {
        let candidate = current.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

fn parse_node_version_file(src: &str) -> Fallible<VersionSpec> {
    let version = src
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .trim_start_matches('v');

    version.parse()
}

#[cfg(test)]
mod tests {
    use super::parse_node_version_file;
    use crate::version::{VersionSpec, VersionTag};
    use node_semver::Version;

    #[test]
    fn parses_exact_version() {
        assert_eq!(
            parse_node_version_file("18.19.0\n").expect("parses"),
            VersionSpec::Exact(Version::parse("18.19.0").expect("valid version"))
        );
    }

    #[test]
    fn parses_v_prefixed_exact_version() {
        assert_eq!(
            parse_node_version_file("v18.19.0\n").expect("parses"),
            VersionSpec::Exact(Version::parse("18.19.0").expect("valid version"))
        );
    }

    #[test]
    fn parses_latest_tag() {
        assert_eq!(
            parse_node_version_file("latest\n").expect("parses"),
            VersionSpec::Tag(VersionTag::Latest)
        );
    }

    #[test]
    fn parses_lts_tag() {
        assert_eq!(
            parse_node_version_file("lts\n").expect("parses"),
            VersionSpec::Tag(VersionTag::Lts)
        );
    }

    #[test]
    fn parses_semver_range() {
        assert!(matches!(
            parse_node_version_file("18\n").expect("parses"),
            VersionSpec::Semver(_)
        ));
    }
}
