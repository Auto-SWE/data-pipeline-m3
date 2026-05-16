use crate::io::{JsonlReader, open_jsonl_writer, read_existing_ids, write_jsonl};
use crate::joern::{ensure_cpg, query_joern};
use crate::prompt::build_feed_text;
use crate::repo_checkout::prepare_repo;
use crate::types::{EnrichedVulnSample, LabelData, ModelInput, RawRecordRow, SourceMeta};
use crate::util::{clean_optional_string, extract_function_name, infer_language};

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "joern-enricher")]
#[command(
    about = "Read PrimeVul JSONL, checkout pre-fix commit, query Joern, write out enrichted JSONL."
)]
pub struct Args {
    /// Input PrimeVul JSONL file.
    #[arg(long)]
    pub input: PathBuf,

    /// Output enriched JSONL file.
    #[arg(long)]
    pub output: PathBuf,

    /// Working directory for repo checkouts, CPG cache, and Joern temporary output.
    #[arg(long, default_value = ".primevul-work")]
    pub workdir: PathBuf,

    /// Path/name of the Joern shell binary.
    #[arg(long, default_value = "joern")]
    pub joern: String,

    /// Path/name of the joern-parse binary.
    #[arg(long, default_value = "joern-parse")]
    pub joern_parse: String,

    /// Joern Scala script used to extract function-level CPG features.
    #[arg(long, default_value = "joern/extract_method_summary.sc")]
    pub joern_script: PathBuf,

    /// Append to output and skip ids already present in output.
    #[arg(long)]
    pub resume: bool,

    /// Process at most N input rows. 0 means no limit.
    #[arg(long, default_value_t = 0)]
    pub limit: usize,

    /// Do not run git/Joern. Useful for testing JSON parsing and prompt generation.
    #[arg(long)]
    pub skip_joern: bool,
}

pub fn run_pipeline(args: Args) -> Result<()> {
    fs::create_dir_all(&args.workdir).context("create working directory")?;

    let done_ids = if args.resume {
        read_existing_ids(&args.output).context("read existing output ids")?
    } else {
        HashSet::new()
    };

    let mut reader = JsonlReader::open(&args.input)?;
    let mut writer = open_jsonl_writer(&args.output, args.resume)?;

    let mut processed: usize = 0;
    while let Some(row) = reader.next_row::<RawRecordRow>()? {
        if args.limit != 0 && processed >= args.limit {
            break;
        }

        let id = format!("primevul-{}", row.idx);

        if done_ids.contains(&id) {
            eprintln!("skip already enriched {id}");
            continue;
        }

        eprintln!("enrich {id} {} {}", row.project, row.commit_id);

        let enriched = enrich_row(&args, &row, id);
        write_jsonl(&mut writer, &enriched)?;

        processed += 1;
    }

    return Ok(());
}

pub fn enrich_row(args: &Args, row: &RawRecordRow, id: String) -> EnrichedVulnSample {
    let mut errors = Vec::new();

    let file_path = clean_optional_string(row.file_name.as_deref());
    let function_name = extract_function_name(&row.func);
    let language = infer_language(file_path.as_deref(), &row.func);

    let mut pre_fix_commit_id = None;
    let mut joern_summary = None;

    if !args.skip_joern {
        match prepare_repo(args, row) {
            Ok(repo) => {
                pre_fix_commit_id = Some(repo.parent_commit.clone());

                match ensure_cpg(args, &repo) {
                    Ok(cpg_path) => {
                        match query_joern(
                            args,
                            &cpg_path,
                            &id,
                            function_name.as_deref(),
                            file_path.as_deref(),
                            &row.func,
                        ) {
                            Ok(summary) => joern_summary = Some(summary),
                            Err(err) => errors.push(format!("joern query failed: {err:#}")),
                        }
                    }
                    Err(err) => errors.push(format!("CPG generation failed: {err:#}")),
                }
            }
            Err(err) => errors.push(format!("CPG generation failed: {err:#}")),
        }
    }

    let model_input = ModelInput {
        language,
        function_name,
        code: row.func.clone(),
        file_path: file_path.clone(),
        joern: joern_summary,
    };

    let feed_text = build_feed_text(&model_input);
    let vulnerable = row.target != 0;

    EnrichedVulnSample {
        id,
        source: SourceMeta {
            dataset: "primevul".to_string(),
            primevul_idx: row.idx,
            project: row.project.clone(),
            project_url: row.project_url.clone(),
            fix_commit_id: row.commit_id.clone(),
            pre_fix_commit_id,
            commit_url: row.commit_url.clone(),
            commit_message: row.commit_message.clone(),
            file_path,
            func_hash: row.func_hash.to_string(),
            file_hash: row.file_hash.clone(),
            cwe: row.cwe.clone(),
            cve: row.cve.clone(),
            cve_desc: row.cve_desc.clone(),
            nvd_url: row.nvd_url.clone(),
        },
        model_input,
        label: LabelData { vulnerable },
        feed_text,
        label_text: if vulnerable { "VULNERABLE" } else { "SAFE" }.to_string(),
        errors,
    }
}
