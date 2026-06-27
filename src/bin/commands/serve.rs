/// HTTP server mode for SLO evaluation (lower priority)
use anyhow::Result;

/// Run the serve command
pub fn run(_bind: &str, _port: u16) -> Result<()> {
    eprintln!("HTTP server mode is not yet implemented.");
    eprintln!("This feature is planned for a future release.");
    eprintln!();
    eprintln!("For now, use the 'eval' subcommand:");
    eprintln!("  neuralbudget eval slo.yaml sample.json");

    Err(anyhow::anyhow!("HTTP server mode not yet implemented"))
}
