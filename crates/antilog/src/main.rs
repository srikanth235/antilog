#![forbid(unsafe_code)]

use clap::Parser;

#[derive(Parser)]
#[command(name = "antilog", version, about = "Drive Antilog from the shell")]
struct Cli {}

fn main() {
    Cli::parse();
}
