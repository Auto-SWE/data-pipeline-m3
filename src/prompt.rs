use crate::types::{JoernSummary, ModelInput};
use std::fmt::{Display, Write as _};

const TASK: &str =
    "Task: Decide whether the following C/C++ function contains a security vulnerability.";
const CODE_SECTION: &str = "[CODE]";
const CPG_SECTION: &str = "[CPG SUMMARY]";
const MAX_CALLS: usize = 80;
const MAX_CONTROL_STRUCTURES: usize = 20;

pub fn build_feed_text(input: &ModelInput) -> String {
    let mut out = String::new();

    write_task_header(&mut out, input);
    write_code_block(&mut out, input);

    if let Some(joern) = &input.joern {
        write_joern_summary(&mut out, joern);
    }

    out
}

fn write_task_header(out: &mut String, input: &ModelInput) {
    line(out, TASK);
    kv(out, "Language", input.language.as_str());

    if let Some(name) = &input.function_name {
        kv(out, "Function", name);
    }
}

fn write_code_block(out: &mut String, input: &ModelInput) {
    let language = input.language.as_str();

    let _ = writeln!(out, "\n{CODE_SECTION}");
    let _ = writeln!(out, "```{language}");
    line(out, input.code.trim());
    line(out, "```");
}

fn write_joern_summary(out: &mut String, j: &JoernSummary) {
    let _ = writeln!(out, "\n{CPG_SECTION}");

    if !j.matched {
        kv(out, "method_match", "not_found");
        return;
    }

    optional_kv(out, "method", j.method_full_name.as_deref());
    write_list(out, "parameters", &j.parameters);
    optional_kv(out, "returns", j.return_type.as_deref());

    kv(out, "cyclomatic_complexity", j.cyclomatic_complexity);
    flag(out, "has_unsafe_c_call", j.has_unsafe_c_call);
    flag(
        out,
        "has_pointer_or_member_access",
        j.has_pointer_or_member_access,
    );
    flag(out, "has_array_indexing", j.has_array_indexing);
    flag(out, "has_address_of", j.has_address_of);
    flag(out, "has_sizeof", j.has_sizeof);

    write_list(out, "unsafe_or_sensitive_calls", &j.unsafe_callees);
    write_limited_list(out, "calls", &j.callees, MAX_CALLS);
    write_bullets(
        out,
        "control_structures",
        &j.control_structures,
        MAX_CONTROL_STRUCTURES,
    );
}

fn line(out: &mut String, value: &str) {
    let _ = writeln!(out, "{value}");
}

fn kv<T: Display>(out: &mut String, key: &str, value: T) {
    let _ = writeln!(out, "{key}: {value}");
}

fn optional_kv(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        if !value.is_empty() {
            kv(out, key, value);
        }
    }
}

fn flag(out: &mut String, key: &str, value: bool) {
    kv(out, key, if value { "yes" } else { "no" });
}

fn write_list(out: &mut String, key: &str, values: &[String]) {
    if !values.is_empty() {
        kv(out, key, values.join(", "));
    }
}

fn write_limited_list(out: &mut String, key: &str, values: &[String], limit: usize) {
    if values.is_empty() {
        return;
    }

    let joined = values
        .iter()
        .take(limit)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    kv(out, key, joined);
}

fn write_bullets(out: &mut String, title: &str, values: &[String], limit: usize) {
    if values.is_empty() {
        return;
    }

    let _ = writeln!(out, "{title}:");

    for value in values.iter().take(limit) {
        let _ = writeln!(out, "- {value}");
    }
}

