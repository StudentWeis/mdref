use clap::Parser;
use serde::Serialize;

mod commands;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,

    /// Show progress bar during operations
    #[arg(long, global = true)]
    progress: bool,
}

#[derive(Serialize)]
struct CommandErrorOutput<'a> {
    operation: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    error: String,
}

fn emit_error(context: &commands::CommandOutputContext, error: &mdref::MdrefError) {
    match context.format {
        commands::OutputFormat::Human => eprintln!("Error: {}", error),
        commands::OutputFormat::Json => {
            let payload = CommandErrorOutput {
                operation: context.operation,
                target: context.target.as_deref(),
                source: context.source.as_deref(),
                destination: context.destination.as_deref(),
                new_name: context.new_name.as_deref(),
                root: context.root.as_deref(),
                dry_run: context.dry_run,
                error: error.to_string(),
            };

            match serde_json::to_string_pretty(&payload) {
                Ok(json) => eprintln!("{json}"),
                Err(json_error) => {
                    eprintln!("Error: {}\nSerialization error: {}", error, json_error)
                }
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let output_context = cli.command.output_context();

    if let Err(e) = commands::handle_command(cli.command, cli.progress) {
        emit_error(&output_context, &e);
        std::process::exit(1);
    }
}
