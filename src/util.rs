use anyhow::{Context, Result, bail};
use std::process::{Command, Output};
use url::Url;

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

pub fn run_and_yield(cmd: &mut Command) -> Result<String> {
    let rendered = format!("{cmd:?}");

    let output: Output = cmd
        .output()
        .with_context(|| format!("failed to start command: {rendered}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        bail!(
            "command failed: {rendered}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            stdout,
            stderr
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn repo_cache_key(project_url: &str, commit_id: &str) -> String {
    let repo_name = repo_name_from_url(project_url).unwrap_or_else(|| "repo".to_string());

    let hash = blake3::hash(format!("{project_url}:{commit_id}").as_bytes());
    let short_hash = &hash.to_hex()[..16];

    sanitize_filename::sanitize(format!("{repo_name}-{short_hash}"))
}

pub fn repo_name_from_url(project_url: &str) -> Option<String> {
    if let Ok(url) = Url::parse(project_url) {
        return url
            .path_segments()?
            .filter(|seg| !seg.is_empty())
            .last()
            .map(|name| name.trim_end_matches(".git").to_string());
    }

    project_url
        .rsplit(['/', ':'])
        .next()
        .map(|name| name.trim_end_matches(".git").to_string())
        .filter(|name| !name.is_empty())
}
