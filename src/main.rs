use clap::Parser;

mod commands;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

fn main() {
    let cli = Cli::parse();
    commands::handle_command(cli.command);
}
