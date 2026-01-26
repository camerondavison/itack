//! Shell completions generation.

use std::io;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli::Cli;
use crate::error::Result;

pub struct CompletionsArgs {
    pub shell: Shell,
}

pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "itack", &mut io::stdout());
    Ok(())
}
