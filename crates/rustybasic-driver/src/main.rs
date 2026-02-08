use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use rustybasic_codegen::{init_all_targets, Codegen, TargetConfig};
use rustybasic_lexer::tokenize;
use rustybasic_parser::parse;
use rustybasic_sema::analyze;

#[derive(Parser)]
#[command(name = "rustybasic")]
#[command(about = "QBASIC compiler for ESP32-C3 (RISC-V)")]
#[command(version)]
struct Cli {
    /// Input .bas source file
    source: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check syntax and types without generating code
    Check,

    /// Dump LLVM IR to stdout
    DumpIr {
        /// Target: "esp32c3" or "host"
        #[arg(long, default_value = "host")]
        target: String,
    },

    /// Compile to object file
    Build {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target: "esp32c3" or "host"
        #[arg(long, default_value = "esp32c3")]
        target: String,
    },

    /// Build ESP-IDF firmware (requires esp-idf toolchain)
    Firmware {
        /// ESP-IDF project directory
        #[arg(long, default_value = "esp-project")]
        project_dir: PathBuf,
    },

    /// Flash firmware to device
    Flash {
        /// Serial port
        #[arg(long)]
        port: String,

        /// ESP-IDF project directory
        #[arg(long, default_value = "esp-project")]
        project_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let source = resolve_includes(&cli.source, &mut HashSet::new())
        .with_context(|| format!("failed to read {}", cli.source.display()))?;

    let mut files = SimpleFiles::new();
    let file_id = files.add(cli.source.display().to_string(), source.clone());

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    // Lex
    let tokens = match tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            let diagnostic = codespan_reporting::diagnostic::Diagnostic::error()
                .with_message(&e.message)
                .with_labels(vec![
                    codespan_reporting::diagnostic::Label::primary(
                        file_id,
                        e.span.start..e.span.end,
                    )
                    .with_message(&e.message),
                ]);
            term::emit(&mut writer.lock(), &config, &files, &diagnostic)?;
            std::process::exit(1);
        }
    };

    // Parse
    let program = match parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            let diagnostic = codespan_reporting::diagnostic::Diagnostic::error()
                .with_message(&e.message)
                .with_labels(vec![
                    codespan_reporting::diagnostic::Label::primary(
                        file_id,
                        e.span.start..e.span.end,
                    )
                    .with_message(&e.message),
                ]);
            term::emit(&mut writer.lock(), &config, &files, &diagnostic)?;
            std::process::exit(1);
        }
    };

    // Semantic analysis
    let sema_result = analyze(&program);
    if sema_result.has_errors() {
        for diag in sema_result.to_diagnostics(file_id) {
            term::emit(&mut writer.lock(), &config, &files, &diag)?;
        }
        std::process::exit(1);
    }

    match cli.command {
        Commands::Check => {
            println!("OK: {} syntax and types valid", cli.source.display());
        }
        Commands::DumpIr { target } => {
            init_all_targets();
            let target_config = parse_target(&target)?;
            let context = inkwell::context::Context::create();
            let mut codegen = Codegen::new(
                &context,
                &cli.source.display().to_string(),
                target_config,
                sema_result,
            );
            codegen.compile(&program)?;
            println!("{}", codegen.dump_ir());
        }
        Commands::Build { output, target } => {
            init_all_targets();
            let target_config = parse_target(&target)?;
            let output = output.unwrap_or_else(|| cli.source.with_extension("o"));
            let context = inkwell::context::Context::create();
            let mut codegen = Codegen::new(
                &context,
                &cli.source.display().to_string(),
                target_config,
                sema_result,
            );
            codegen.compile(&program)?;
            codegen.write_object_file(&output)?;
            println!("Compiled to {}", output.display());
        }
        Commands::Firmware { project_dir } => {
            init_all_targets();
            let target_config = TargetConfig::esp32c3();
            let obj_path = project_dir.join("main").join("basic_program.o");

            let context = inkwell::context::Context::create();
            let mut codegen = Codegen::new(
                &context,
                &cli.source.display().to_string(),
                target_config,
                sema_result,
            );
            codegen.compile(&program)?;
            codegen.write_object_file(&obj_path)?;
            println!("Object file: {}", obj_path.display());

            // Invoke idf.py build
            let status = Command::new("idf.py")
                .current_dir(&project_dir)
                .args(["build"])
                .status()
                .context("failed to run idf.py — is ESP-IDF installed?")?;

            if !status.success() {
                anyhow::bail!("idf.py build failed");
            }
            println!("Firmware built successfully!");
        }
        Commands::Flash { port, project_dir } => {
            let status = Command::new("idf.py")
                .current_dir(&project_dir)
                .args(["flash", "-p", &port])
                .status()
                .context("failed to run idf.py — is ESP-IDF installed?")?;

            if !status.success() {
                anyhow::bail!("idf.py flash failed");
            }
            println!("Flashed successfully!");
        }
    }

    Ok(())
}

/// Recursively resolve `INCLUDE "file.bas"` directives by inlining the
/// included file's contents. Paths are resolved relative to the directory
/// of the file that contains the directive. Circular includes are detected
/// via a visited set of canonical paths.
fn resolve_includes(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("cannot resolve path: {}", path.display()))?;

    if !visited.insert(canonical.clone()) {
        anyhow::bail!("circular INCLUDE detected: {}", path.display());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut output = String::with_capacity(content.len());

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(included_file) = parse_include_directive(trimmed) {
            let include_path = base_dir.join(included_file);
            let included_source = resolve_includes(&include_path, visited)?;
            output.push_str(&included_source);
            if !included_source.ends_with('\n') {
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    visited.remove(&canonical);
    Ok(output)
}

/// Parse an `INCLUDE "filename"` directive (case-insensitive).
/// Returns `Some(filename)` if the line matches, `None` otherwise.
fn parse_include_directive(line: &str) -> Option<&str> {
    let line = line.trim();
    let rest = line.strip_prefix("INCLUDE")
        .or_else(|| line.strip_prefix("include"))
        .or_else(|| {
            // General case-insensitive check
            if line.len() >= 7 && line[..7].eq_ignore_ascii_case("INCLUDE") {
                Some(&line[7..])
            } else {
                None
            }
        })?;

    let rest = rest.trim();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let filename = &rest[..end];
    if filename.is_empty() {
        return None;
    }
    Some(filename)
}

fn parse_target(target: &str) -> Result<TargetConfig> {
    match target {
        "esp32c3" | "esp32-c3" | "riscv" => Ok(TargetConfig::esp32c3()),
        "host" | "native" => Ok(TargetConfig::host()),
        _ => anyhow::bail!(
            "unknown target '{target}'. Valid targets: esp32c3, host"
        ),
    }
}
