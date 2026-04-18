use clap::Parser;

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

fn main() {
    let cli = Cli::parse();
    if let Err(e) = commands::handle_command(cli.command, cli.progress) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
