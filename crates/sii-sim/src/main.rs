//! Binary entry point for the SII simulator CLI server (`sii-sim`).

#![forbid(unsafe_code)]

use clap::Parser;
use sii_sim::{HttpServer, Scenario, ScenarioConfig, SiiSimulator};
use std::fs;
use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

/// Simulador NO oficial del SII de Chile (Unofficial Chilean SII simulator).
#[derive(Debug, Parser)]
#[command(
    name = "sii-sim",
    version,
    about = "Simulador local de servicios y endpoints del SII de Chile (no oficial)"
)]
struct Cli {
    /// Dirección de escucha en loopback (solo 127.0.0.1).
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Escenario de simulación: aceptado, rechazado, `en_proceso`, `token_invalido`, `upload_rechazado`, `semilla_error`.
    #[arg(long, default_value = "aceptado")]
    scenario: Scenario,

    /// Ruta a archivo de configuración de escenarios en JSON.
    #[arg(long)]
    scenario_file: Option<String>,

    /// Modo silencioso (no imprime solicitudes ni respuestas).
    #[arg(long)]
    quiet: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = if let Some(ref path) = cli.scenario_file {
        match fs::read_to_string(path) {
            Ok(content) => match ScenarioConfig::from_json(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "error al parsear archivo de escenario '{path}': {e}"
                    );
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "error al leer archivo de escenario '{path}': {e}"
                );
                return ExitCode::FAILURE;
            }
        }
    } else {
        ScenarioConfig::new(cli.scenario)
    };

    let server = match HttpServer::bind(&cli.listen) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "error al vincular servidor en '{listen}': {e}",
                listen = cli.listen
            );
            return ExitCode::FAILURE;
        }
    };

    let bound_addr = match server.local_addr() {
        Ok(addr) => addr.to_string(),
        Err(_) => cli.listen.clone(),
    };

    let default_scen = config.default_scenario;
    let simulator = Arc::new(SiiSimulator::new().with_config(config));

    if !cli.quiet {
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(
            stdout,
            "sii-sim iniciado en http://{bound_addr} (escenario: {default_scen})"
        );
        let _ = writeln!(stdout, "Presiona Ctrl+C para detener.");
    }

    let quiet = cli.quiet;
    let observer_fn = |req: &sii::transport::Request, resp: &sii::transport::Response| {
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "<- {} {}", req.method, req.url);
        let _ = writeln!(stdout, "-> {} ({} bytes)", resp.status, resp.body.len());
    };
    let observer: Option<&sii_sim::http::Observer> = if quiet { None } else { Some(&observer_fn) };

    if let Err(e) = server.run_blocking(&simulator, observer) {
        let _ = writeln!(std::io::stderr().lock(), "error en el servidor: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
