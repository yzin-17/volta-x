use crate::command::Command;
use std::path::PathBuf;
use volta_core::directory_platform::{DirectoryPlatformSpec, DirectoryPlatforms};
use volta_core::error::{ErrorKind, ExitCode, Fallible};
use volta_core::session::{ActivityKind, Session};

#[derive(clap::Args)]
pub(crate) struct Unuse {
    /// Tools to remove from the current directory mapping, like `node`, `npm`, `pnpm`, or `yarn`.
    #[arg(value_name = "tool", required_unless_present = "all")]
    tools: Vec<String>,

    /// Removes all tool versions set for the current directory
    #[arg(long, conflicts_with = "tools")]
    all: bool,

    /// Directory whose local tool versions should be removed
    #[arg(long, value_name = "dir")]
    dir: Option<PathBuf>,
}

impl Command for Unuse {
    fn run(self, session: &mut Session) -> Fallible<ExitCode> {
        session.add_event_start(ActivityKind::Unuse);

        let mut platforms = DirectoryPlatforms::current()?;

        if self.all {
            if let Some(directory) = self.dir {
                platforms.clear_dir(&directory)?;
                println!("Cleared tool versions for {}.", directory.display());
            } else {
                platforms.clear_current_dir()?;
                println!("Cleared tool versions for the current directory.");
            }
        } else {
            let tools = parse_tools(&self.tools)?;
            let update = |platform: &mut DirectoryPlatformSpec| {
                for tool in tools {
                    tool.unset(platform);
                }
            };

            if let Some(directory) = self.dir {
                platforms.unset_dir(&directory, update)?;
                println!(
                    "Removed selected tool versions for {}.",
                    directory.display()
                );
            } else {
                platforms.unset_current_dir(update)?;
                println!("Removed selected tool versions for the current directory.");
            }
        }

        session.add_event_end(ActivityKind::Unuse, ExitCode::Success);
        Ok(ExitCode::Success)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DirectoryTool {
    Node,
    Npm,
    Pnpm,
    Yarn,
}

impl DirectoryTool {
    pub(crate) fn parse(tool: &str) -> Fallible<Self> {
        match tool {
            "node" => Ok(DirectoryTool::Node),
            "npm" => Ok(DirectoryTool::Npm),
            "pnpm" => Ok(DirectoryTool::Pnpm),
            "yarn" => Ok(DirectoryTool::Yarn),
            name => Err(ErrorKind::InvalidToolName {
                name: name.to_string(),
                errors: vec!["expected one of: node, npm, pnpm, yarn".into()],
            }
            .into()),
        }
    }

    fn unset(self, platform: &mut DirectoryPlatformSpec) {
        match self {
            DirectoryTool::Node => platform.node = None,
            DirectoryTool::Npm => platform.npm = None,
            DirectoryTool::Pnpm => platform.pnpm = None,
            DirectoryTool::Yarn => platform.yarn = None,
        }
    }
}

fn parse_tools(tools: &[String]) -> Fallible<Vec<DirectoryTool>> {
    tools
        .iter()
        .map(|tool| DirectoryTool::parse(tool))
        .collect()
}
