use crate::cli::Commands;
use anyhow::Result;

use clap::{CommandFactory, Parser};
use std::process::ExitCode;

mod api;
mod badge;
mod cli;
mod config;
mod currency;
mod repo;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, arg_required_else_help = true)]
struct CommandLine {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let args = CommandLine::parse();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Dispatch command
    match &args.command {
        Some(Commands::Serve(args)) => crate::cli::serve(args).await,
        // `arg_required_else_help` makes clap print help and exit before we get
        // here when no subcommand is given, but handle it gracefully instead of
        // panicking in case that guard is ever removed.
        None => {
            CommandLine::command().print_help()?;
            Ok(ExitCode::FAILURE)
        }
    }
}
