mod api;
mod auth;
mod config;
mod tui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "un",
    about = "Update Night — browse AI dev tools from your terminal",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Authorize this device with your Update Night account
    Login,
    /// Remove stored credentials
    Logout,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Login) => auth::device_login().await,
        Some(Command::Logout) => config::logout(),
        None => tui::run().await,
    }
}
