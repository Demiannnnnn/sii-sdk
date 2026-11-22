//! Binary entry point for the Chilean SII CLI (`sii`).
//!
//! Provides command line operations for building, stamping, digitally signing,
//! uploading DTE documents, querying processing statuses, and generating ledgers.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod args;
mod commands;
mod error;
mod output;

use args::{Cli, Commands, DteCommands, EnvioCommands, LibroCommands};
use clap::{CommandFactory, Parser};
use error::CliError;
use output::print_error;
use std::io::Write;
use std::process::ExitCode;

/// Main entry point for the `sii` CLI binary.
fn main() -> ExitCode {
    let cli = Cli::parse();
    let is_json = cli.json;
    let is_quiet = cli.quiet;

    let result = run_command(&cli, is_json, is_quiet);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            print_error(&err, is_json);
            ExitCode::from(1)
        }
    }
}

fn run_command(cli: &Cli, is_json: bool, is_quiet: bool) -> Result<(), CliError> {
    let Some(command) = &cli.command else {
        let mut stdout = std::io::stdout().lock();
        let _ = Cli::command().write_help(&mut stdout);
        let _ = writeln!(stdout);
        return Ok(());
    };

    match command {
        Commands::Dte(dte_args) => match &dte_args.command {
            DteCommands::Build(args) => commands::dte::run_dte_build(args, is_json, is_quiet),
            DteCommands::Timbrar(args) => commands::dte::run_dte_timbrar(args, is_json, is_quiet),
            DteCommands::Firmar(args) => commands::dte::run_dte_firmar(args, is_json, is_quiet),
        },
        Commands::Envio(envio_args) => match &envio_args.command {
            EnvioCommands::Enviar(args) => {
                commands::envio::run_envio_enviar(args, is_json, is_quiet)
            }
            EnvioCommands::Estado(args) => {
                commands::envio::run_envio_estado(args, is_json, is_quiet)
            }
        },
        Commands::Libro(libro_args) => match &libro_args.command {
            LibroCommands::Build(args) => commands::libro::run_libro_build(args, is_json, is_quiet),
        },
    }
}
