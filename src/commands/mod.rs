//! Command implementations.

pub mod board;
pub mod claim;
pub mod completions;
pub mod create;
pub mod depend;
pub mod doctor;
pub mod done;
pub mod edit;
pub mod init;
pub mod list;
pub mod release;
pub mod search;
pub mod set_session;
pub mod show;
pub mod undepend;

use crate::cli::{Cli, Commands};
use crate::error::Result;
use crate::output::OutputFormat;

/// Dispatch a command based on CLI arguments.
pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run(),

        Commands::Create {
            title,
            epic,
            body,
            message,
            depends_on,
        } => create::run(create::CreateArgs {
            title,
            epic,
            body,
            message,
            depends_on,
        }),

        Commands::Show { id, json } => show::run(show::ShowArgs {
            id,
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Table
            },
        }),

        Commands::Edit { id, body, message } => edit::run(edit::EditArgs { id, body, message }),

        Commands::Done { id } => done::run(done::DoneArgs { id }),

        Commands::Claim {
            id,
            assignee,
            session,
        } => claim::run(claim::ClaimArgs {
            id,
            assignee,
            session,
        }),

        Commands::Release { id } => release::run(release::ReleaseArgs { id }),

        Commands::Depend { id, deps } => depend::run(depend::DependArgs { id, deps }),

        Commands::Undepend { id, deps } => undepend::run(undepend::UndependArgs { id, deps }),

        Commands::SetSession { id, session } => {
            set_session::run(set_session::SetSessionArgs { id, session })
        }

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

        Commands::Doctor => doctor::run(),

        Commands::Search {
            query,
            all_branches,
            json,
        } => search::run(search::SearchArgs {
            query,
            all_branches,
            format: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Table
            },
        }),

        Commands::Completions { shell } => completions::run(completions::CompletionsArgs { shell }),
    }
}
