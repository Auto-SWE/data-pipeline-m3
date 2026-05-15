use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRecordRow {
    pub idx: u64,
    pub project: String,
    pub commit_id: String,
    pub project_url: String,
    pub commit_url: String,
    pub commit_message: Option<String>,
    pub target: i32,
    pub func: String,
    pub func_hash: u128,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub cwe: Vec<String>,
    pub cve: Option<String>,
    pub cve_desc: Option<String>,
    pub nvd_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedVulnSample {
    pub id: String,

    /// Keep this for tracing/dedup/debug. Do not feed this block to the model.
    pub source: SourceMeta,

    /// Structured model input. This is safe to serialize into feed_text.
    pub model_input: ModelInput,

    /// Label used by the trainer.
    pub label: LabelData,

    /// Exact text to pass to a causal LM / instruction model.
    pub feed_text: String,

    /// Expected completion for a causal LM fine-tune.
    pub label_text: String,

    /// Non-fatal pipeline errors. Rows are still emitted so the job is resumable.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMeta {
    pub dataset: String,
    pub primevul_idx: u64,
    pub project: String,
    pub project_url: String,
    pub fix_commit_id: String,
    pub pre_fix_commit_id: Option<String>,
    pub commit_url: String,
    pub commit_message: Option<String>,
    pub file_path: Option<String>,
    pub func_hash: String,
    pub file_hash: Option<String>,
    pub cwe: Vec<String>,
    pub cve: Option<String>,
    pub cve_desc: Option<String>,
    pub nvd_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    pub language: Language,
    pub function_name: Option<String>,
    pub code: String,
    pub file_path: Option<String>,
    pub joern: Option<JoernSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelData {
    pub target: i32,
    pub vulnerable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    C,
    Cpp,
    Unknown,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Unknown => "c",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoernSummary {
    pub matched: bool,
    pub matched_methods_count: usize,
    pub method_full_name: Option<String>,
    pub method_file: Option<String>,
    pub line_number: Option<u32>,
    pub return_type: Option<String>,
    pub parameters: Vec<String>,
    pub local_types: Vec<String>,
    pub callees: Vec<String>,
    pub unsafe_callees: Vec<String>,
    pub operators: Vec<String>,
    pub control_structures: Vec<String>,
    pub cyclomatic_complexity: u32,
    pub has_unsafe_c_call: bool,
    pub has_pointer_or_member_access: bool,
    pub has_array_indexing: bool,
    pub has_address_of: bool,
    pub has_sizeof: bool,
}

#[derive(Debug, Clone)]
pub struct RepoCheckout {
    pub key: String,
    pub path: PathBuf,
    pub parent_commit: String,
}
