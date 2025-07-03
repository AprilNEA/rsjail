use anyhow::Result;
use clap::Parser;
use std::fs;

mod config;
mod jail;

use config::JailConfig;
use jail::Jail;

#[derive(Parser)]
#[command(name = "rsjail")]
#[command(about = "A simple jail implementation in Rust")]
struct Args {
    #[arg(short, long)]
    config: String,
    
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.verbose {
        env_logger::init();
    }
    
    // Check if running with root privileges
    if !nix::unistd::getuid().is_root() {
        eprintln!("Error: This program must be run as root");
        std::process::exit(1);
    }
    
    // Read config file
    let config_content = fs::read_to_string(&args.config)?;
    let config: JailConfig = serde_json::from_str(&config_content)?;
    
    // Create and run jail
    let jail = Jail::new(config);
    jail.run()?;
    
    Ok(())
}
