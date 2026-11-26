use super::suite::{SuiteReport, SuiteRunContext, apply_suite_report};
use super::xml::{JunitXmlOptions, parse_junit_xml};
use super::{
    CommandOutput, OUTPUT_LIMIT, command_result, parse_simple_command, read_bounded_file,
    run_headless, run_headless_spawn,
};
use crate::execution::environment;
use crate::runtime::{HostTool, PreflightReport, RuntimeSource, find_host_executable};
use crate::{CheckResult, CheckStatus, CourseLanguage, DomainError};
use std::ffi::OsString;
use std::path::PathBuf;

const XML_OPTIONS: JunitXmlOptions = JunitXmlOptions {
    strip_empty_parens: false,
    display_separator: " > ",
};

pub(super) const DEFAULT_COMMAND: &str = "bun test";
pub(super) const DEFAULT_TIMEOUT_SEC: u64 = 5;
pub(super) const REPORT_LABEL: &str = "jsTsTestReport";

enum JsTsRunner {
    Bun(PathBuf),
    Node {
        executable: PathBuf,
        needs_strip_types: bool,
    },
}

enum DirectInvocation {
    BunTest(Vec<String>),
    NodeTest(Vec<String>),
}

pub(super) fn run(ctx: SuiteRunContext<'_>) -> Result<CheckResult, DomainError> {
    let direct = if !ctx.explicit_command {
        Some(DirectInvocation::BunTest(Vec::new()))
    } else {
        classify_direct_command(ctx.script)
    };

    if let Some(invocation) = direct
        && let Some(runner) = resolve_runner(ctx.report, &invocation)
    {
        return run_direct(ctx, runner, invocation);
    }

    let output = run_headless(
        ctx.script,
        ctx.directory,
        ctx.report,
        Some(CourseLanguage::JsTs),
        ctx.timeout,
        ctx.cancelled,
    )?;
    let mut result = command_result(ctx.check, output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }
    let suite_report = parse_output_report(&result.raw_output);
    apply_suite_report(
        &mut result,
        suite_report,
        REPORT_LABEL,
        &ctx.check.require_tests,
    );
    Ok(result)
}

fn run_direct(
    ctx: SuiteRunContext<'_>,
    runner: JsTsRunner,
    invocation: DirectInvocation,
) -> Result<CheckResult, DomainError> {
    let temp = tempfile::tempdir().map_err(super::execution_error)?;
    let report_path = temp.path().join("junit.xml");
    let env = environment(ctx.report, Some(CourseLanguage::JsTs))?;

    let (program, args) = match (runner, invocation) {
        (JsTsRunner::Bun(exe), DirectInvocation::BunTest(extra)) => {
            let mut args = vec![OsString::from("test")];
            args.extend(extra.into_iter().map(OsString::from));
            args.push(OsString::from("--reporter=junit"));
            args.push(OsString::from(format!(
                "--reporter-outfile={}",
                report_path.display()
            )));
            (exe, args)
        }
        (
            JsTsRunner::Node {
                executable,
                needs_strip_types,
            },
            DirectInvocation::BunTest(extra) | DirectInvocation::NodeTest(extra),
        ) => {
            let mut args = Vec::new();
            if needs_strip_types {
                args.push(OsString::from("--experimental-strip-types"));
            }
            args.push(OsString::from("--test"));
            args.push(OsString::from("--test-reporter=junit"));
            args.push(OsString::from(format!(
                "--test-reporter-destination={}",
                report_path.display()
            )));
            args.push(OsString::from("--test-reporter=spec"));
            args.push(OsString::from("--test-reporter-destination=stdout"));
            args.extend(extra.into_iter().map(OsString::from));
            (executable, args)
        }
        (JsTsRunner::Bun(exe), DirectInvocation::NodeTest(extra)) => {
            let mut args = vec![OsString::from("test")];
            args.extend(extra.into_iter().map(OsString::from));
            args.push(OsString::from("--reporter=junit"));
            args.push(OsString::from(format!(
                "--reporter-outfile={}",
                report_path.display()
            )));
            (exe, args)
        }
    };

    let output: CommandOutput = run_headless_spawn(
        program,
        args,
        ctx.directory,
        env,
        ctx.timeout,
        ctx.cancelled,
    )?;
    let mut result = command_result(ctx.check, output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }

    let xml_contents = read_bounded_file(&report_path, OUTPUT_LIMIT).ok();
    let mut suite_report = match xml_contents.as_deref() {
        Some(xml) if !xml.trim().is_empty() => parse_junit_xml(xml, XML_OPTIONS),
        _ => parse_output_report(&result.raw_output),
    };
    if !suite_report.passed()
        && suite_report.compile_error.is_none()
        && let Some(syntax_err) =
            extract_syntax_error(&result.raw_output, suite_report.tests.is_empty())
    {
        suite_report.completed_successfully = false;
        suite_report.compile_error = Some(syntax_err);
    }
    apply_suite_report(
        &mut result,
        suite_report,
        REPORT_LABEL,
        &ctx.check.require_tests,
    );
    Ok(result)
}

fn parse_output_report(raw_output: &str) -> SuiteReport {
    let mut report = parse_junit_xml(raw_output, XML_OPTIONS);
    if !report.passed() && report.compile_error.is_none() {
        report.compile_error = extract_syntax_error(raw_output, report.tests.is_empty());
    }
    report
}

fn classify_direct_command(script: &str) -> Option<DirectInvocation> {
    let words = parse_simple_command(script)?;
    let first = words.first()?.as_str();
    let bin = first
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    let bin = bin.strip_suffix(".exe").unwrap_or(&bin);
    if bin == "bun" && words.get(1).map(String::as_str) == Some("test") {
        let rest = &words[2..];
        if rest
            .iter()
            .any(|arg| arg == "--reporter" || arg.starts_with("--reporter="))
        {
            return None;
        }
        return Some(DirectInvocation::BunTest(rest.to_vec()));
    }
    if bin == "node" {
        let mut idx = 1;
        if words.get(idx).map(String::as_str) == Some("--experimental-strip-types") {
            idx += 1;
        }
        if words.get(idx).map(String::as_str) == Some("--test") {
            let rest = &words[idx + 1..];
            if rest
                .iter()
                .any(|arg| arg == "--test-reporter" || arg.starts_with("--test-reporter="))
            {
                return None;
            }
            return Some(DirectInvocation::NodeTest(rest.to_vec()));
        }
    }
    None
}

fn resolve_runner(report: &PreflightReport, invocation: &DirectInvocation) -> Option<JsTsRunner> {
    let row = report
        .languages
        .iter()
        .find(|row| row.language == CourseLanguage::JsTs);

    let find_node = || {
        let node_check =
            row.and_then(|row| row.system.iter().find(|check| check.tool == HostTool::Node));
        let exe = node_check
            .and_then(|check| check.executable.clone())
            .filter(|path| path.is_file())
            .or_else(|| find_host_executable("node"))?;
        let needs_strip_types = node_check
            .and_then(|check| check.value.as_deref())
            .map(node_needs_strip_types_flag)
            .unwrap_or_else(|| probe_node_needs_strip_types(&exe));
        Some(JsTsRunner::Node {
            executable: exe,
            needs_strip_types,
        })
    };

    let find_bun = || {
        if let Some(row) = row {
            if row.selected == Some(RuntimeSource::Managed)
                && let Some(managed) = &row.managed
                && managed.executable.is_file()
            {
                return Some(JsTsRunner::Bun(managed.executable.clone()));
            }
            if let Some(bun_check) = row.system.iter().find(|check| check.tool == HostTool::Bun)
                && let Some(exe) = &bun_check.executable
                && exe.is_file()
            {
                return Some(JsTsRunner::Bun(exe.clone()));
            }
        }
        find_host_executable("bun").map(JsTsRunner::Bun)
    };

    match invocation {
        DirectInvocation::NodeTest(_) => find_node(),
        DirectInvocation::BunTest(args) => {
            let has_bun_flags = args.iter().any(|arg| arg.starts_with('-'));
            if let Some(row) = row {
                match row.selected {
                    Some(RuntimeSource::Managed) => {
                        return row
                            .managed
                            .as_ref()
                            .filter(|managed| managed.executable.is_file())
                            .map(|managed| JsTsRunner::Bun(managed.executable.clone()));
                    }
                    Some(RuntimeSource::System {
                        tool: HostTool::Bun,
                    }) => {
                        if let Some(runner) = find_bun() {
                            return Some(runner);
                        }
                    }
                    Some(RuntimeSource::System {
                        tool: HostTool::Node,
                    }) => {
                        if !has_bun_flags {
                            return find_node();
                        }
                        return find_bun();
                    }
                    _ => {}
                }
            }
            if let Some(bun) = find_bun() {
                return Some(bun);
            }
            if !has_bun_flags {
                return find_node();
            }
            None
        }
    }
}

fn probe_node_needs_strip_types(executable: &std::path::Path) -> bool {
    let Ok(output) = std::process::Command::new(executable)
        .arg("--version")
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let version = String::from_utf8_lossy(&output.stdout);
    node_needs_strip_types_flag(&version)
}

fn node_needs_strip_types_flag(version: &str) -> bool {
    let trimmed = version.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    major == 22 && (6..18).contains(&minor)
}

fn extract_syntax_error(raw_output: &str, no_tests_recorded: bool) -> Option<String> {
    let cleaned = strip_ansi_escapes::strip_str(raw_output);
    let syntax_markers = [
        "SyntaxError:",
        "SyntaxError [",
        "error: Unexpected",
        "# Unhandled error between tests",
        "Cannot find module",
        "ERR_MODULE_NOT_FOUND",
        "ERR_INVALID_TYPESCRIPT_SYNTAX",
    ];
    let module_eval_markers = ["ReferenceError:", "TypeError:"];
    let matched = syntax_markers.iter().any(|marker| cleaned.contains(marker))
        || (no_tests_recorded
            && module_eval_markers
                .iter()
                .any(|marker| cleaned.contains(marker)));
    if matched {
        let excerpt: Vec<&str> = cleaned
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(12)
            .collect();
        if !excerpt.is_empty() {
            return Some(excerpt.join("\n"));
        }
    }
    None
}
