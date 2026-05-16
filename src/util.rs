use anyhow::{Context, Result, bail};
use regex::Regex;
use std::path::Path;
use std::process::{Command, Output};
use url::Url;

use crate::types::Language;

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

pub fn clean_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| !s.eq_ignore_ascii_case("none"))
        .filter(|s| !s.eq_ignore_ascii_case("null"))
        .map(ToOwned::to_owned)
}

pub fn extract_function_name(code: &str) -> Option<String> {
    let re = Regex::new(
        r"(?m)([A-Za-z_~][A-Za-z0-9_:~]*)\s*\([^;{}]*\)\s*(?:const\s*)?(?:noexcept\s*)?(?:->\s*[^{}]+)?\{",
    )
    .ok()?;

    let raw = re.captures(code)?.get(1)?.as_str();
    let short = raw.rsplit("::").next().unwrap_or(raw).trim();

    if short.is_empty() {
        None
    } else {
        Some(short.trim_start_matches('~').to_string())
    }
}

pub fn infer_language(file_path: Option<&str>, code: &str) -> Language {
    if let Some(path) = file_path.map(|p| p.to_ascii_lowercase()) {
        if path.ends_with(".cpp")
            || path.ends_with(".cc")
            || path.ends_with(".cxx")
            || path.ends_with(".hpp")
            || path.ends_with(".hh")
            || path.ends_with(".hxx")
        {
            return Language::Cpp;
        }

        if path.ends_with(".c") || path.ends_with(".h") {
            return Language::C;
        }
    }

    if code.contains("::")
        || code.contains("template <")
        || code.contains("std::")
        || code.contains("class ")
        || code.contains("namespace ")
    {
        Language::Cpp
    } else {
        Language::C
    }
}

pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn is_unsafe_callee(name: &str) -> bool {
    let n = name
        .rsplit("::")
        .next()
        .unwrap_or(name)
        .rsplit('.')
        .next()
        .unwrap_or(name)
        .trim();

    matches!(
        n,
        "gets"
            | "strcpy"
            | "strncpy"
            | "strcat"
            | "strncat"
            | "sprintf"
            | "vsprintf"
            | "scanf"
            | "sscanf"
            | "fscanf"
            | "memcpy"
            | "memmove"
            | "memset"
            | "malloc"
            | "calloc"
            | "realloc"
            | "free"
            | "read"
            | "recv"
            | "recvfrom"
            | "fread"
    )
}
