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
    error: String,
}

fn emit_error(
    format: commands::OutputFormat,
    operation: &str,
    target: Option<&str>,
    error: &mdref::MdrefError,
) {
    match format {
        commands::OutputFormat::Human => eprintln!("Error: {}", error),
        commands::OutputFormat::Json => {
            let payload = CommandErrorOutput {
                operation,
                target,
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

    let error_format = cli.command.output_format();
    let command_name = cli.command.name();
    let command_target = cli.command.target().map(str::to_owned);

    if let Err(e) = commands::handle_command(cli.command, cli.progress) {
        emit_error(error_format, command_name, command_target.as_deref(), &e);
        std::process::exit(1);
    }
}
