// Based on https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/tools/jq5
// Original: Copyright 2021 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use clap::Parser;
use futures::future::join_all;
use json5format::{FormatOptions, Json5Format, ParsedDocument};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

mod reader;
mod traverser;

/// Spawns a `jq` process with the specified filter and extra args, pipes `json_string` into its stdin.
async fn run_jq(
    filter: &str,
    json_string: String,
    jq_path: &Option<PathBuf>,
    jq_args: &[String],
) -> Result<String, anyhow::Error> {
    let jq_bin = match jq_path {
        Some(path) => {
            let command_str = path.as_path().to_str().unwrap();
            if !Path::exists(Path::new(command_str)) {
                return Err(anyhow::anyhow!(
                    "Path provided in --path-to-jq did not specify a valid path to a binary."
                ));
            }
            command_str.to_string()
        }
        None => "jq".to_string(),
    };

    let mut child = Command::new(&jq_bin)
        .args(jq_args)
        .arg(filter)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "jq not found in PATH. Install jq or use --path-to-jq to specify its location."
                )
            } else {
                anyhow::anyhow!(e)
            }
        })?;

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(json_string.as_bytes()).await?;
    stdin.flush().await?;
    drop(stdin);

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            return Err(anyhow::anyhow!("jq error:\n{}", stderr));
        }
        return Err(anyhow::anyhow!(
            "jq returned non-zero exit code: {}",
            output.status
        ));
    }

    Ok(String::from_utf8(output.stdout)?)
}

/// Calls jq on the provided json and then fills back comments at correct places.
async fn run_jq5(
    filter: &str,
    parsed_json5: ParsedDocument,
    json_string: String,
    jq_path: &Option<PathBuf>,
    jq_args: &[String],
    json5_output: bool,
) -> Result<String, anyhow::Error> {
    let jq_out = run_jq(filter, json_string, jq_path, jq_args).await?;
    if !json5_output {
        return Ok(jq_out);
    }
    let mut parsed_json = ParsedDocument::from_string(jq_out.clone(), None);
    match parsed_json {
        Ok(ref mut doc) => {
            // Try to fill comments; ignore mismatch errors (e.g. object→primitive)
            let _ = traverser::fill_comments(&parsed_json5.content, &mut doc.content);
            let format = Json5Format::with_options(FormatOptions {
                indent_by: 2,
                ..Default::default()
            })?;
            Ok(format.to_string(doc)?)
        }
        Err(_) => {
            // jq output is not valid JSON5 (e.g., raw string/number) — return as-is
            Ok(jq_out)
        }
    }
}

/// Calls `run_jq5` on the contents of a file and returns the result.
async fn run_jq5_on_file(
    filter: &str,
    file: &PathBuf,
    jq_path: &Option<PathBuf>,
    jq_args: &[String],
    json5_output: bool,
) -> Result<String, anyhow::Error> {
    let (parsed_json5, json_string) = reader::read_json5_fromfile(file)?;
    run_jq5(filter, parsed_json5, json_string, jq_path, jq_args, json5_output).await
}

/// Processes multiple files concurrently via `join_all`.
async fn run(
    filter: &str,
    files: &[PathBuf],
    jq_path: &Option<PathBuf>,
    jq_args: &[String],
    json5_output: bool,
) -> Result<Vec<String>, anyhow::Error> {
    let futures: Vec<_> = files
        .iter()
        .map(|file| run_jq5_on_file(filter, file, jq_path, jq_args, json5_output))
        .collect();
    let results = join_all(futures).await;
    let mut outputs = Vec::with_capacity(results.len());
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Error processing file '{}':\n{}",
                    files[i].display(),
                    err
                ));
            }
            Ok(output) => outputs.push(output),
        }
    }
    Ok(outputs)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Opt::parse();

    if args.files.is_empty() {
        let (parsed_json5, json_string) = reader::read_json5_from_input(&mut io::stdin())?;
        let out = run_jq5(&args.filter, parsed_json5, json_string, &args.jq_path, &args.jq_args, args.json5).await?;
        io::stdout().write_all(out.as_bytes())?;
    } else {
        let outs = run(&args.filter, &args.files, &args.jq_path, &args.jq_args, args.json5).await?;
        for out in outs {
            io::stdout().write_all(out.as_bytes())?;
        }
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = "jq5",
    about = "An extension of jq to work on JSON5 objects, preserving comments.\n\nRequires jq to be installed and available in PATH (or specify --path-to-jq)."
)]
struct Opt {
    /// The jq filter expression
    filter: String,

    /// JSON5 files to process (reads stdin if none provided)
    files: Vec<PathBuf>,

    /// Path to the jq binary (defaults to 'jq' in PATH)
    #[arg(long = "path-to-jq")]
    jq_path: Option<PathBuf>,

    /// Output JSON5 format with comment preservation (default: JSON output like jq)
    #[arg(long)]
    json5: bool,

    /// Extra arguments to pass through to jq (e.g. --arg, --argjson, --slurp)
    #[arg(last = true)]
    jq_args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jq_available() -> bool {
        std::process::Command::new("jq")
            .arg("--version")
            .output()
            .is_ok()
    }

    #[tokio::test]
    async fn run_jq_id_filter_1() {
        if !jq_available() {
            eprintln!("skipping test: jq not found");
            return;
        }
        let filter = ".";
        let input = String::from("{}");
        assert_eq!(run_jq(filter, input, &None, &[]).await.unwrap().trim(), "{}");
    }

    #[tokio::test]
    async fn run_jq_id_filter_2() {
        if !jq_available() {
            eprintln!("skipping test: jq not found");
            return;
        }
        let filter = ".";
        let input = String::from(r#"{"foo": 1, "bar": 2}"#);
        let result = run_jq(filter, input, &None, &[]).await.unwrap();
        assert!(result.contains("\"foo\": 1"));
        assert!(result.contains("\"bar\": 2"));
    }

    #[tokio::test]
    async fn run_jq_deconstruct_filter() {
        if !jq_available() {
            eprintln!("skipping test: jq not found");
            return;
        }
        let filter = "{foo2: .foo1, bar2: .bar1}";
        let input = String::from(r#"{"foo1": 0, "bar1": 42}"#);
        let result = run_jq(filter, input, &None, &[]).await.unwrap();
        assert!(result.contains("\"foo2\": 0"));
        assert!(result.contains("\"bar2\": 42"));
    }

    #[tokio::test]
    async fn run_jq5_deconstruct_filter() {
        if !jq_available() {
            eprintln!("skipping test: jq not found");
            return;
        }
        let filter = "{foo: .foo, baz: .bar}";
        let json5_string = String::from(
            r##"{
  //Foo
  foo: 0,
  //Bar
  bar: 42
}"##,
        );
        let (parsed_json5, json_string) = reader::read_json5(json5_string).unwrap();
        let result = run_jq5(filter, parsed_json5, json_string, &None, &[], true).await.unwrap();
        assert!(result.contains("foo"));
        assert!(result.contains("baz"));
    }
}
