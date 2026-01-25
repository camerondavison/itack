//! Command implementations.

pub mod board;
pub mod claim;
pub mod create;
pub mod done;
pub mod edit;
pub mod init;
pub mod list;
pub mod release;
pub mod show;

use crate::cli::{Cli, Commands};
use crate::error::Result;
use crate::output::OutputFormat;

/// Dispatch a command based on CLI arguments.
pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(),

        Commands::Create { title, epic, body } => {
            create::run(create::CreateArgs { title, epic, body })
        }

        Commands::Show { id, json } => show::run(show::ShowArgs {
            id,
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Table
            },
        }),

        Commands::Edit { id } => edit::run(edit::EditArgs { id }),

        Commands::Done { id } => done::run(done::DoneArgs { id }),

        Commands::Claim { id, assignee } => claim::run(claim::ClaimArgs { id, assignee }),

        Commands::Release { id } => release::run(release::ReleaseArgs { id }),

        Commands::List {
            status,
            epic,
            assignee,
            json,
        } => list::run(list::ListArgs {
            status,
            epic,
            assignee,
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Table
            },
        }),

        Commands::Board { json } => board::run(board::BoardArgs {
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Table
            },
        }),
    }
}
