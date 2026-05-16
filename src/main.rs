mod io;
mod joern;
mod pipeline;
mod prompt;
mod repo_checkout;
mod types;
mod util;

use crate::pipeline::run_pipeline;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = pipeline::Args::parse();
    run_pipeline(args)
}
