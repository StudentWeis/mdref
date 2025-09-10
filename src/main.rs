use clap::Parser;

mod commands;

#[derive(Parser)]
#[command(name = "mdref")]
#[command(about = "A tool to find and manage Markdown link references")]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

fn main() {
    let cli = Cli::parse();
    commands::handle_command(cli.command);
}
