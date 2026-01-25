//! itack - Git-backed issue tracker CLI.

use std::process::ExitCode;

use clap::Parser;

mod cli;
mod commands;
mod core;
mod error;
mod output;
mod storage;

use cli::Cli;
use error::exit_codes;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match commands::dispatch(cli) {
        Ok(()) => ExitCode::from(exit_codes::SUCCESS),
        Err(e) => {
            eprintln!("Error: {}", e);
            e.exit_code()
        }
    }
}
