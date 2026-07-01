use volta_core::error::{ExitCode, Fallible};
use volta_core::session::{ActivityKind, Session};
use volta_core::tool::Spec;

use crate::command::Command;

#[derive(clap::Args)]
pub(crate) struct Default {
    /// Tools to set as defaults, like `node@18.19.0`, `npm@bundled`, or `yarn@1.22.22`.
    #[arg(value_name = "tool[@version]", required = true)]
    tools: Vec<String>,
}

impl Command for Default {
    fn run(self, session: &mut Session) -> Fallible<ExitCode> {
        session.add_event_start(ActivityKind::Default);

        for tool in Spec::from_strings(&self.tools, "default")? {
            tool.default(session)?;
        }

        session.add_event_end(ActivityKind::Default, ExitCode::Success);
        Ok(ExitCode::Success)
    }
}
