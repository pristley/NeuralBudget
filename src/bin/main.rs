/// NeuralBudget CLI Tool
/// 
/// Command-line interface for SLO evaluation, configuration validation, and rule generation.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod commands;
use commands::{check, convert, eval, gen_rules, serve};

#[derive(Parser)]
#[command(name = "neuralbudget")]
#[command(about = "SLO evaluation and configuration tool", long_about = None)]
#[command(version = "0.2.0")]
#[command(author = "NeuralBudget Contributors")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate an SLO against a sample
    ///
    /// Load a YAML SLO config and JSON sample, evaluate the SLO, and output the result.
    ///
    /// Example: neuralbudget eval slo.yaml sample.json
    Eval {
        /// Path to SLO configuration (YAML)
        config: PathBuf,

        /// Path to metric sample (JSON)
        sample: PathBuf,

        /// Output as JSON instead of human-readable format
        #[arg(long)]
        json: bool,

        /// Verbose output with detailed metrics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Generate Prometheus recording and alerting rules from SLO config
    ///
    /// Creates PrometheusRule objects compatible with Prometheus Operator.
    /// Can be applied directly to Kubernetes clusters.
    ///
    /// Example: neuralbudget gen-rules slo.yaml > rules.yaml
    ///          kubectl apply -f rules.yaml
    GenRules {
        /// Path to SLO configuration (YAML)
        config: PathBuf,

        /// Output as Kubernetes CRD (PrometheusRule) instead of plain YAML
        #[arg(long)]
        kubernetes: bool,

        /// Namespace for Kubernetes resources
        #[arg(long, default_value = "monitoring")]
        namespace: String,
    },

    /// Validate SLO configuration and check for common mistakes
    ///
    /// Performs schema validation and warns about unrealistic thresholds,
    /// missing alert destinations, and other configuration issues.
    ///
    /// Example: neuralbudget check slo.yaml
    Check {
        /// Path to SLO configuration (YAML)
        config: PathBuf,

        /// Fail on warnings (exit code 1) instead of just logging them
        #[arg(long)]
        strict: bool,
    },

    /// Convert between SLO formats (NeuralBudget ↔ OpenSLO)
    ///
    /// Enables migration from/to OpenSLO (CNCF standard format) for vendor-neutral
    /// SLO definitions. Supports bidirectional conversion.
    ///
    /// Example: neuralbudget convert --from openslo input.yaml --to neuralbudget > output.yaml
    ///          neuralbudget convert --from neuralbudget slo.yaml --to openslo > output-openslo.yaml
    Convert {
        /// Path to input SLO file
        #[arg(long)]
        input: PathBuf,

        /// Input format: 'openslo', 'slo', 'neuralbudget', 'nb'
        #[arg(long)]
        from: String,

        /// Output format: 'openslo', 'slo', 'neuralbudget', 'nb'
        #[arg(long)]
        to: String,

        /// Service name (required for conversion to OpenSLO)
        #[arg(long)]
        service: Option<String>,

        /// SLO name (defaults to 'converted-slo')
        #[arg(long, default_value = "converted-slo")]
        name: String,
    },

    /// Start HTTP server for SLO evaluation
    ///
    /// Accepts POST /eval with sample + config in request body.
    /// Returns evaluation result as JSON.
    ///
    /// Example: neuralbudget serve --port 8080
    ///          curl -X POST http://localhost:8080/eval \\
    ///            -d @sample.json -H "Content-Type: application/json"
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Address to bind to
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Eval {
            config,
            sample,
            json,
            verbose,
        } => eval::run(&config, &sample, json, verbose),

        Commands::GenRules {
            config,
            kubernetes,
            namespace,
        } => gen_rules::run(&config, kubernetes, &namespace),

        Commands::Check { config, strict } => check::run(&config, strict),

        Commands::Convert {
            input,
            from,
            to,
            service,
            name,
        } => {
            let from_fmt = convert::Format::from_str(&from)
                .map_err(|e| anyhow::anyhow!("Invalid source format: {}", e))?;
            let to_fmt = convert::Format::from_str(&to)
                .map_err(|e| anyhow::anyhow!("Invalid target format: {}", e))?;

            let svc_name = service.unwrap_or_else(|| "default-service".to_string());

            match convert::run(&input, from_fmt, to_fmt, &svc_name, &name) {
                Ok(output) => {
                    println!("{}", output);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        Commands::Serve { port, bind } => serve::run(&bind, port),
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        eprintln!("\nFor more help, run: neuralbudget --help");
        process::exit(1);
    }
    
    Ok(())
}
