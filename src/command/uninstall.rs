use volta_core::error::{ErrorKind, ExitCode, Fallible};
use volta_core::session::{ActivityKind, Session};
use volta_core::tool;
use volta_core::version::VersionSpec;

use crate::command::Command;

#[derive(clap::Args)]
pub(crate) struct Uninstall {
    /// The tool to uninstall, like `node@18.19.0`, `yarn@1.22.22`, or `typescript`.
    tool: String,
}

impl Command for Uninstall {
    fn run(self, session: &mut Session) -> Fallible<ExitCode> {
        session.add_event_start(ActivityKind::Uninstall);

        let tool = tool::Spec::try_from_str(&self.tool)?;

        // Package uninstalls still remove the active package shim/config, not a
        // versioned package image. Runtime and package-manager specs continue
        // to the versioned uninstall implementation.
        if let tool::Spec::Package(_name, version) = &tool {
            let VersionSpec::None = version else {
                return Err(ErrorKind::Unimplemented {
                    feature: "uninstalling specific versions of tools".into(),
                }
                .into());
            };
        }

        tool.uninstall(session)?;

        session.add_event_end(ActivityKind::Uninstall, ExitCode::Success);
        Ok(ExitCode::Success)
    }
}
