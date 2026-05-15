use anyhow::{Context, Result, bail};
use std::process::{Command, Output};

pub fn run_ok(cmd: &mut Command) -> Result<()> {
    let rendered = format!("{cmd:?}");

    let output: Output = cmd
        .output()
        .with_context(|| format!("failed to start command: {rendered}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    bail!(
        "command failed: {rendered}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout,
        stderr
    );
}
