use crate::pipeline::Args;
use crate::types::{RawRecordRow, RepoCheckout};
use crate::util::{repo_cache_key, run_and_yield, run_ok};

use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub fn prepare_repo(args: &Args, row: &RawRecordRow) -> Result<RepoCheckout> {
    let key = repo_cache_key(&row.project_url, &row.commit_id);
    let repo_dir = args.workdir.join("repos").join(&key);
    let marker = repo_dir.join(".primevul_pre_fix_commit");

    if marker.exists() && repo_dir.join(".git").exists() {
        let parent_commit = fs::read_to_string(&marker)
            .with_context(|| format!("read marker {:?}", marker))?
            .trim()
            .to_string();

        return Ok(RepoCheckout {
            key,
            path: repo_dir,
            parent_commit,
        });
    }

    if repo_dir.exists() {
        fs::remove_dir_all(&repo_dir)
            .with_context(|| format!("remove partial repo {:?}", repo_dir))?;
    }

    fs::create_dir_all(&repo_dir).with_context(|| format!("create repo dir {:?}", repo_dir))?;

    run_ok(
        Command::new("git")
            .arg("init")
            .arg(".")
            .current_dir(&repo_dir),
    )?;

    run_ok(
        Command::new("git")
            .args(["remote", "add", "origin"])
            .arg(&row.project_url)
            .current_dir(&repo_dir),
    )?;

    let filtered_fetch = run_ok(
        Command::new("git")
            .arg("fetch")
            .arg("--depth=2")
            .arg("--filter=blob:none")
            .arg("origin")
            .arg(&row.commit_id)
            .current_dir(&repo_dir),
    );

    if let Err(err) = filtered_fetch {
        eprintln!(
            "warning: filtered fetch failed for {}; retrying without --filter: {err:#}",
            row.commit_id
        );

        run_ok(
            Command::new("git")
                .arg("fetch")
                .arg("--depth=2")
                .arg("origin")
                .arg(&row.commit_id)
                .current_dir(&repo_dir),
        )?;
    }

    run_ok(
        Command::new("git")
            .args(["checkout", "--detach"])
            .arg(format!("{}^", row.commit_id))
            .current_dir(&repo_dir),
    )
    .with_context(|| format!("checkout parent of fixing commit {}", row.commit_id))?;

    let parent_commit = run_and_yield(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo_dir),
    )?
    .trim()
    .to_string();

    fs::write(&marker, format!("{parent_commit}\n"))
        .with_context(|| format!("write marker {:?}", marker))?;

    Ok(RepoCheckout {
        key,
        path: repo_dir,
        parent_commit: parent_commit,
    })
}
