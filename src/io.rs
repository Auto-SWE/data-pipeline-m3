use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
};

pub struct JsonlReader {
    lines: Lines<BufReader<File>>,
    line_no: usize,
}

impl JsonlReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("open input {:?}", path))?;

        Ok(Self {
            lines: BufReader::new(file).lines(),
            line_no: 0,
        })
    }

    pub fn next_row<T>(&mut self) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        loop {
            let Some(line) = self.lines.next() else {
                return Ok(None);
            };

            self.line_no += 1;

            let line = line.with_context(|| format!("read line {}", self.line_no))?;

            if line.trim().is_empty() {
                continue;
            }

            let row = serde_json::from_str::<T>(&line)
                .with_context(|| format!("parse JSONL at line {}", self.line_no))?;

            return Ok(Some(row));
        }
    }

    pub fn open_jsonl_writer(path: &Path, append: bool) -> Result<BufWriter<File>> {
        let mut open = OpenOptions::new();
        open.create(true).write(true);

        if append {
            open.append(true);
        } else {
            open.truncate(true);
        }

        let file = open
            .open(path)
            .with_context(|| format!("open output {:?}", path))?;

        Ok(BufWriter::new(file))
    }

    pub fn write_jsonl<T, W>(writer: &mut W, value: &T) -> Result<()>
    where
        T: Serialize,
        W: Write,
    {
        serde_json::to_writer(&mut *writer, value).context("write JSON object")?;
        writeln!(writer).context("write JSONL newline")?;
        writer.flush().context("flush JSONL writer")?;

        Ok(())
    }

    pub fn read_existing_ids(path: &Path) -> Result<HashSet<String>> {
        let mut ids = HashSet::new();

        if !path.exists() {
            return Ok(ids);
        }

        let file = File::open(path).with_context(|| format!("open existing output {:?}", path))?;

        for line in BufReader::new(file).lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
                    ids.insert(id.to_string());
                }
            }
        }

        Ok(ids)
    }
}
