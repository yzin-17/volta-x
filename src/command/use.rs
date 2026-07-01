use crate::command::Command;
use node_semver::Version;
use volta_core::directory_platform::{DirectoryPlatformSpec, DirectoryPlatforms};
use volta_core::error::{ExitCode, Fallible};
use volta_core::session::{ActivityKind, Session};
use volta_core::tool::Spec;

#[derive(clap::Args)]
pub(crate) struct Use {
    /// Tools to use in the current directory, like `node@18.19.0`, `npm@bundled`, or `yarn@1.22.22`.
    #[arg(value_name = "tool[@version]", required = true)]
    tools: Vec<String>,
}

impl Command for Use {
    fn run(self, session: &mut Session) -> Fallible<ExitCode> {
        session.add_event_start(ActivityKind::Use);

        if self.tools.len() == 1 && self.tools[0] == "list" {
            list_directory_platforms()?;
        } else {
            for tool in Spec::from_strings(&self.tools, "use")? {
                tool.use_current_dir(session)?;
            }
        }

        session.add_event_end(ActivityKind::Use, ExitCode::Success);
        Ok(ExitCode::Success)
    }
}

fn list_directory_platforms() -> Fallible<()> {
    let platforms = DirectoryPlatforms::current()?;
    let mut found = false;

    for (directory, platform) in platforms.entries() {
        found = true;
        println!("{} {}", directory.display(), format_platform(platform));
    }

    if !found {
        println!("No directory tool versions configured.");
    }

    Ok(())
}

fn format_platform(platform: &DirectoryPlatformSpec) -> String {
    let mut tools = Vec::new();

    if let Some(node) = &platform.node {
        tools.push(format_tool("node", node));
    }
    if let Some(npm) = &platform.npm {
        match npm {
            Some(npm) => tools.push(format_tool("npm", npm)),
            None => tools.push("npm@bundled".to_string()),
        }
    }
    if let Some(pnpm) = &platform.pnpm {
        tools.push(format_tool("pnpm", pnpm));
    }
    if let Some(yarn) = &platform.yarn {
        tools.push(format_tool("yarn", yarn));
    }

    tools.join(" ")
}

fn format_tool(name: &str, version: &Version) -> String {
    format!("{name}@{version}")
}
