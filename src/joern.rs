use crate::pipeline::Args;
use crate::types::{JoernSummary, RepoCheckout};
use crate::util::{is_unsafe_callee, path_to_string, run_ok};

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn ensure_cpg(args: &Args, repo: &RepoCheckout) -> Result<PathBuf> {
    let cpg_dir = args.workdir.join("cpg");
    fs::create_dir_all(&cpg_dir).context("create cpg dir")?;

    let short_parent = &repo.parent_commit[..repo.parent_commit.len().min(12)];
    let cpg_path = cpg_dir.join(format!("{}-{}.bin.zip", repo.key, short_parent));

    if cpg_path.exists() {
        return Ok(cpg_path);
    }

    let first = run_ok(
        Command::new(&args.joern_parse)
            .arg(&repo.path)
            .arg("--output")
            .arg(&cpg_path),
    );

    if let Err(first_err) = first {
        let first_err = format!("{first_err:#}");
        let _ = fs::remove_file(&cpg_path);

        run_ok(
            Command::new(&args.joern_parse)
                .arg(&repo.path)
                .arg("--out")
                .arg(&cpg_path),
        )
        .with_context(|| format!("joern-parse failed with --ouput first: {first_err}"))?;
    }

    let metadata =
        fs::metadata(&cpg_path).with_context(|| format!("read CPG metadata {:?}", cpg_path))?;

    if metadata.len() == 0 {
        bail!("joern-parse created an empty CPG at {:?}", cpg_path);
    }

    Ok(cpg_path)
}

pub fn query_joern(
    args: &Args,
    cpg_path: &Path,
    id: &str,
    function_name: Option<&str>,
    file_path: Option<&str>,
    raw_code: &str,
) -> Result<JoernSummary> {
    if !args.joern_script.exists() {
        bail!("Joern script not found at {:?}", args.joern_script);
    }

    let out_dir = args.workdir.join("joern-output");
    fs::create_dir_all(&out_dir).context("create joern-output dir")?;

    let out_file = out_dir.join(format!("{}.tsv", sanitize_filename::sanitize(id)));
    let function_name = function_name.unwrap_or("");
    let file_path = file_path.unwrap_or("");

    let _ = fs::remove_file(&out_file);

    run_ok(
        Command::new(&args.joern)
            .arg("--script")
            .arg(&args.joern_script)
            .arg("--param")
            .arg(format!("cpgFile={}", path_to_string(cpg_path)))
            .arg("--param")
            .arg(format!("functionName={function_name}"))
            .arg("--param")
            .arg(format!("filePath={file_path}"))
            .arg("--param")
            .arg(format!("outFile={}", path_to_string(&out_file))),
    )?;

    if !out_file.exists() {
        bail!(
            "Joern finished but did not create output file {:?}",
            out_file
        );
    }
    let fields = parse_joern_tsv(&out_file)?;
    let matched = fields.get("FOUND").map(|s| s == "true").unwrap_or(false);
    let callees = parse_list(fields.get("CALLS"));

    let unsafe_callees: Vec<String> = callees
        .iter()
        .filter(|callee| is_unsafe_callee(callee))
        .cloned()
        .collect();

    let operators = parse_list(fields.get("OPERATORS"));
    let control_structures = parse_list(fields.get("CONTROL_STRUCTURES"));
    let cyclomatic_complexity = fields
        .get("CONTROL_STRUCTURE_COUNT")
        .and_then(|s| s.parse::<u32>().ok())
        .map(|n| n + 1)
        .unwrap_or(1);

    let has_pointer_or_member_access = raw_code.contains("->")
        || raw_code.contains('*')
        || operators.iter().any(|op| {
            op.contains("indirection")
                || op.contains("indirect")
                || op.contains("fieldAccess")
                || op.contains("memberAccess")
        });

    let has_array_indexing = raw_code.contains('[')
        || operators
            .iter()
            .any(|op| op.contains("indexAccess") || op.contains("computedMemberAccess"));

    let has_address_of = raw_code.contains('&')
        || operators
            .iter()
            .any(|op| op.contains("addressOf") || op.contains("address"));

    let has_sizeof = raw_code.contains("sizeof")
        || fields
            .get("CALLS")
            .map(|s| s.contains("sizeof"))
            .unwrap_or(false);

    let has_unsafe_c_call = !unsafe_callees.is_empty();

    Ok(JoernSummary {
        matched,
        matched_methods_count: fields
            .get("MATCHED_METHODS_COUNT")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        method_full_name: opt_field(fields.get("METHOD_FULL_NAME")),
        method_file: opt_field(fields.get("METHOD_FILE")),
        line_number: fields.get("LINE_NUMBER").and_then(|s| s.parse().ok()),
        return_type: opt_field(fields.get("RETURN_TYPE")),
        parameters: parse_list(fields.get("PARAMETERS")),
        local_types: parse_list(fields.get("LOCAL_TYPES")),
        callees,
        unsafe_callees,
        operators,
        control_structures,
        cyclomatic_complexity,
        has_unsafe_c_call,
        has_pointer_or_member_access,
        has_array_indexing,
        has_address_of,
        has_sizeof,
    })
}

fn parse_joern_tsv(path: &Path) -> Result<BTreeMap<String, String>> {
    let f = File::open(path).with_context(|| format!("open Joern tsv {:?}", path))?;
    let mut map = BTreeMap::new();

    for line in BufReader::new(f).lines() {
        let line = line?;
        if let Some((k, v)) = line.split_once('\t') {
            map.insert(k.to_string(), v.to_string());
        }
    }

    Ok(map)
}

fn parse_list(value: Option<&String>) -> Vec<String> {
    value
        .map(|s| {
            s.split('|')
                .map(str::trim)
                .filter(|x| !x.is_empty() && *x != "none")
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn opt_field(value: Option<&String>) -> Option<String> {
    value
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "none")
        .map(ToOwned::to_owned)
}
