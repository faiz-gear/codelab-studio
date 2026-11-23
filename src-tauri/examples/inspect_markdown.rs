use std::env;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use codelab_studio_lib::course::{parse_markdown, parse_markdown_file, ParseMarkdownOptions};
use codelab_studio_lib::domain::course::ParsedCourse;
use codelab_studio_lib::domain::ids::RelWorkspacePath;
use codelab_studio_lib::domain::pty::PtyRole;

enum OutputMode {
    Report,
    Json,
    Bench(usize),
}

struct CliArgs {
    path: PathBuf,
    mode: OutputMode,
}

fn parse_cli_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut path: Option<PathBuf> = None;
    let mut mode = OutputMode::Report;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => {
                mode = OutputMode::Json;
            }
            "--bench" => {
                let raw_runs = args
                    .next()
                    .ok_or_else(|| "missing integer argument after --bench".to_owned())?;
                let runs = raw_runs
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --bench runs '{raw_runs}': {err}"))?;
                if runs == 0 {
                    return Err("--bench runs must be >= 1".to_owned());
                }
                mode = OutputMode::Bench(runs);
            }
            "--help" | "-h" => {
                return Err(usage_text());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag '{other}'\n\n{}", usage_text()));
            }
            positional => {
                if path.is_some() {
                    return Err(format!(
                        "unexpected extra argument '{positional}'\n\n{}",
                        usage_text()
                    ));
                }
                path = Some(PathBuf::from(positional));
            }
        }
    }

    let path = path.ok_or_else(usage_text)?;
    Ok(CliArgs { path, mode })
}

fn usage_text() -> String {
    concat!(
        "Usage: cargo run --manifest-path src-tauri/Cargo.toml ",
        "--example inspect_markdown -- <markdown-path> [--json | --bench <runs>]"
    )
    .to_owned()
}

fn format_role(role: &PtyRole) -> String {
    match role {
        PtyRole::Daemon {
            service_key,
            label,
            port_hint,
        } => {
            let port = port_hint
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_owned());
            format!("daemon(key={service_key}, label={label}, port={port})")
        }
        PtyRole::Oneshot { slot_index } => format!("oneshot(slot={slot_index})"),
    }
}

fn print_report(course: &ParsedCourse) {
    println!("Title: {}", course.title);
    println!("Course ID: {}", course.course_id);
    println!("Language: {:?}", course.language);
    println!("Intro Nodes: {}", course.intro_nodes.len());
    println!("Chapters: {}", course.chapters.len());

    for chapter in &course.chapters {
        println!();
        println!(
            "[{}] {} ({})",
            chapter.index, chapter.title, chapter.chapter_id.0
        );
        if let Some(goal) = &chapter.goal {
            println!("  Goal: {goal}");
        }
        println!(
            "  Commands: total={} primary={} secondary={}",
            chapter.command_blocks.len(),
            chapter.three_step.step1_primary_commands.len(),
            chapter.three_step.secondary_commands.len(),
        );
        for cmd in &chapter.command_blocks {
            let cwd = cmd
                .cwd
                .as_ref()
                .map(RelWorkspacePath::as_str)
                .unwrap_or("<workspace-root>");
            println!(
                "    - {} | secondary={} | cwd={} | role={}",
                cmd.id.0,
                cmd.is_secondary,
                cwd,
                format_role(&cmd.role),
            );
        }
        println!(
            "  Reference Code Blocks: {}",
            chapter.three_step.step2_reference_blocks.len()
        );
        for block in &chapter.three_step.step2_reference_blocks {
            let lang = block.language.as_deref().unwrap_or("-");
            let target = block
                .target_path
                .as_ref()
                .map(RelWorkspacePath::as_str)
                .unwrap_or("-");
            println!(
                "    - #{} | lang={} | target={}",
                block.code_index, lang, target
            );
        }
        println!(
            "  Self-Checks: {} | Reflections: {}",
            chapter.three_step.step3_self_checks.len(),
            chapter.three_step.reflection_nodes.len(),
        );
    }
}

fn run_bench(path: &Path, runs: usize) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
    let line_count = source.lines().count();
    let byte_size = source.len();
    let options = ParseMarkdownOptions::default();

    let mut timings_ms = Vec::with_capacity(runs);
    for run_idx in 0..runs {
        let started = Instant::now();
        let parsed = parse_markdown(black_box(&source), black_box(&options))
            .map_err(|err| format!("parse failed on run {}: {err}", run_idx + 1))?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

        let chapter_count = parsed.chapters.len();
        let command_count: usize = parsed
            .chapters
            .iter()
            .map(|chapter| chapter.command_blocks.len())
            .sum();
        if chapter_count == 0 {
            return Err(format!(
                "benchmark expected non-empty chapters on run {}",
                run_idx + 1
            ));
        }

        println!(
            "run {:>2}: {:>7.3} ms (chapters={}, commands={})",
            run_idx + 1,
            elapsed_ms,
            chapter_count,
            command_count
        );
        timings_ms.push(elapsed_ms);
    }

    timings_ms.sort_by(f64::total_cmp);
    let min_ms = timings_ms[0];
    let max_ms = timings_ms[timings_ms.len() - 1];
    let median_ms = if timings_ms.len() % 2 == 1 {
        timings_ms[timings_ms.len() / 2]
    } else {
        let mid = timings_ms.len() / 2;
        (timings_ms[mid - 1] + timings_ms[mid]) / 2.0
    };

    println!();
    println!(
        "Summary: runs={runs} lines={line_count} bytes={byte_size} \
         min={min_ms:.3}ms median={median_ms:.3}ms max={max_ms:.3}ms"
    );

    Ok(())
}

fn main() -> ExitCode {
    let cli = match parse_cli_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    match cli.mode {
        OutputMode::Bench(runs) => {
            if let Err(err) = run_bench(&cli.path, runs) {
                eprintln!("{err}");
                return ExitCode::FAILURE;
            }
        }
        OutputMode::Json | OutputMode::Report => {
            let course = match parse_markdown_file(&cli.path, &ParseMarkdownOptions::default()) {
                Ok(parsed) => parsed,
                Err(err) => {
                    eprintln!("parse error: {err}");
                    return ExitCode::FAILURE;
                }
            };
            if matches!(cli.mode, OutputMode::Json) {
                match serde_json::to_string_pretty(&course) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("serialization error: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                print_report(&course);
            }
        }
    }

    ExitCode::SUCCESS
}
