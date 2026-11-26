use super::suite::{SuiteCaseId, SuiteReport, SuiteRunContext, apply_suite_report};
use super::{
    OUTPUT_LIMIT, command_result, normalize_executable_name, parse_simple_command,
    read_bounded_file, run_headless, run_headless_spawn,
};
use crate::execution::environment;
use crate::runtime::{PreflightReport, RuntimeSource, find_host_executable};
use crate::{CheckResult, CheckStatus, CourseLanguage, DomainError, TestCaseResult};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

pub(super) const DEFAULT_COMMAND: &str = "pytest --json-report";
pub(super) const DEFAULT_TIMEOUT_SEC: u64 = 8;
pub(super) const REPORT_LABEL: &str = "pytestReport";

#[derive(Deserialize)]
struct PytestJsonReport {
    exitcode: Option<i64>,
    #[serde(default)]
    collectors: Vec<PytestCollector>,
    tests: Option<Vec<PytestTest>>,
}

#[derive(Deserialize)]
struct PytestCollector {
    nodeid: Option<String>,
    outcome: Option<String>,
    longrepr: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct PytestTest {
    nodeid: String,
    outcome: String,
    setup: Option<PytestPhase>,
    call: Option<PytestPhase>,
    teardown: Option<PytestPhase>,
}

#[derive(Deserialize)]
struct PytestPhase {
    duration: Option<f64>,
    outcome: Option<String>,
    crash: Option<PytestCrash>,
    longrepr: Option<serde_json::Value>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Deserialize)]
struct PytestCrash {
    message: Option<String>,
}

enum DirectPytestInvocation {
    Pytest(Vec<String>),
    UvRunPytest(Vec<String>),
}

pub(super) fn run(ctx: SuiteRunContext<'_>) -> Result<CheckResult, DomainError> {
    let direct = if !ctx.explicit_command {
        Some(DirectPytestInvocation::Pytest(Vec::new()))
    } else {
        classify_direct_pytest_args(ctx.script)
    };

    if let Some(invocation) = direct
        && let Some((program, mut base_args, extra_args)) =
            resolve_pytest_runner(ctx.report, invocation)
    {
        let temp = tempfile::tempdir().map_err(super::execution_error)?;
        let report_path = temp.path().join("report.json");
        let env = environment(ctx.report, Some(CourseLanguage::Python))?;

        let mut has_json_flag = false;
        for arg in extra_args {
            if arg == "--json-report" {
                has_json_flag = true;
            }
            base_args.push(OsString::from(arg));
        }
        if !has_json_flag {
            base_args.push(OsString::from("--json-report"));
        }
        base_args.push(OsString::from(format!(
            "--json-report-file={}",
            report_path.display()
        )));

        let output = run_headless_spawn(
            program,
            base_args,
            ctx.directory,
            env,
            ctx.timeout,
            ctx.cancelled,
        )?;
        let mut result = command_result(ctx.check, output);
        if result.status == CheckStatus::TimedOut {
            return Ok(result);
        }

        let json_contents = read_bounded_file(&report_path, OUTPUT_LIMIT).ok();
        let mut suite_report = match json_contents.as_deref() {
            Some(json) if !json.trim().is_empty() => parse_report(json),
            _ => parse_report(&result.raw_output),
        };
        if !suite_report.passed()
            && suite_report.compile_error.is_none()
            && suite_report.tests.is_empty()
        {
            suite_report.compile_error = extract_python_error(&result.raw_output);
        }
        apply_suite_report(
            &mut result,
            suite_report,
            REPORT_LABEL,
            &ctx.check.require_tests,
        );
        return Ok(result);
    }

    let output = run_headless(
        ctx.script,
        ctx.directory,
        ctx.report,
        Some(CourseLanguage::Python),
        ctx.timeout,
        ctx.cancelled,
    )?;
    let mut result = command_result(ctx.check, output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }
    let mut suite_report = parse_report(&result.raw_output);
    if !suite_report.passed()
        && suite_report.compile_error.is_none()
        && suite_report.tests.is_empty()
    {
        suite_report.compile_error = extract_python_error(&result.raw_output);
    }
    apply_suite_report(
        &mut result,
        suite_report,
        REPORT_LABEL,
        &ctx.check.require_tests,
    );
    Ok(result)
}

pub(super) fn parse_report(input: &str) -> SuiteReport {
    let trimmed = input.trim_start_matches('\u{feff}').trim();
    if !trimmed.starts_with('{') {
        return SuiteReport::incomplete();
    }
    let Ok(parsed) = serde_json::from_str::<PytestJsonReport>(trimmed) else {
        return SuiteReport::incomplete();
    };
    let Some(raw_tests) = parsed.tests else {
        return SuiteReport::incomplete();
    };

    let mut collector_errors = Vec::new();
    for collector in &parsed.collectors {
        if matches!(collector.outcome.as_deref(), Some("failed" | "error")) {
            let detail = collector
                .longrepr
                .as_ref()
                .and_then(format_longrepr)
                .or_else(|| collector.nodeid.clone())
                .or_else(|| collector.outcome.clone())
                .unwrap_or_else(|| "error".into());
            collector_errors.push(strip_ansi_escapes::strip_str(detail));
        }
    }

    let mut tests = BTreeMap::new();
    let mut complete = collector_errors.is_empty();
    if let Some(code) = parsed.exitcode
        && !matches!(code, 0 | 1)
    {
        complete = false;
    }

    for item in raw_tests {
        let nodeid = item.nodeid.trim();
        if nodeid.is_empty() {
            complete = false;
            continue;
        }
        let (package, test_name) = match nodeid.split_once("::") {
            Some((pkg, name)) if !name.is_empty() => (pkg.to_owned(), name.to_owned()),
            _ => (String::new(), nodeid.to_owned()),
        };
        let status = match item.outcome.as_str() {
            "passed" => CheckStatus::Passed,
            "skipped" | "xfailed" => CheckStatus::Skipped,
            "failed" | "error" => CheckStatus::Failed,
            "xpassed" => {
                if item
                    .call
                    .as_ref()
                    .and_then(|phase| phase.outcome.as_deref())
                    == Some("failed")
                {
                    CheckStatus::Failed
                } else {
                    CheckStatus::Passed
                }
            }
            _ => {
                complete = false;
                CheckStatus::Failed
            }
        };

        let mut total_secs = 0.0_f64;
        for phase in [&item.setup, &item.call, &item.teardown]
            .into_iter()
            .flatten()
        {
            if let Some(secs) = phase.duration
                && secs.is_finite()
                && secs >= 0.0
            {
                total_secs += secs;
            }
        }
        let duration_ms = (total_secs * 1000.0).round() as u64;
        let output = collect_test_output(&item);

        let id = SuiteCaseId {
            package,
            test: test_name,
        };
        let case = TestCaseResult {
            name: nodeid.to_owned(),
            status,
            duration_ms,
            output,
        };
        tests.insert(id, case);
    }

    SuiteReport {
        completed_successfully: complete,
        tests,
        compile_error: (!collector_errors.is_empty()).then(|| collector_errors.join("\n\n")),
    }
}

fn collect_test_output(item: &PytestTest) -> String {
    let mut parts = Vec::new();
    for phase in [&item.setup, &item.call, &item.teardown]
        .into_iter()
        .flatten()
    {
        let crash = phase
            .crash
            .as_ref()
            .and_then(|crash| crash.message.as_deref())
            .map(str::trim)
            .filter(|msg| !msg.is_empty());
        let longrepr = phase
            .longrepr
            .as_ref()
            .and_then(format_longrepr)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        match (crash, longrepr) {
            (Some(msg), Some(repr)) => {
                if repr.contains(msg) {
                    parts.push(repr);
                } else {
                    parts.push(format!("{msg}\n{repr}"));
                }
            }
            (Some(msg), None) => parts.push(msg.to_owned()),
            (None, Some(repr)) => parts.push(repr),
            (None, None) => {}
        }
        for stream in [&phase.stdout, &phase.stderr].into_iter().flatten() {
            let trimmed = stream.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_owned());
            }
        }
    }
    strip_ansi_escapes::strip_str(parts.join("\n"))
}

fn format_longrepr(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(items) => {
            let joined = items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (!joined.is_empty()).then_some(joined)
        }
        serde_json::Value::Object(map) => map
            .get("reprcrash")
            .and_then(|crash| crash.get("message"))
            .and_then(|msg| msg.as_str())
            .map(ToOwned::to_owned)
            .or_else(|| Some(value.to_string())),
        _ => None,
    }
}

fn classify_direct_pytest_args(script: &str) -> Option<DirectPytestInvocation> {
    let words = parse_simple_command(script)?;
    let bin = normalize_executable_name(words.first()?);
    let (is_uv_run, rest) = if bin == "pytest" {
        (false, &words[1..])
    } else if matches!(bin.as_str(), "python" | "python3")
        && words.get(1).map(String::as_str) == Some("-m")
        && words.get(2).map(String::as_str) == Some("pytest")
    {
        (false, &words[3..])
    } else if bin == "uv"
        && words.get(1).map(String::as_str) == Some("run")
        && words.get(2).map(String::as_str) == Some("pytest")
    {
        (true, &words[3..])
    } else {
        return None;
    };
    if rest
        .iter()
        .any(|arg| arg == "--json-report-file" || arg.starts_with("--json-report-file="))
    {
        return None;
    }
    if is_uv_run {
        Some(DirectPytestInvocation::UvRunPytest(rest.to_vec()))
    } else {
        Some(DirectPytestInvocation::Pytest(rest.to_vec()))
    }
}

fn resolve_pytest_runner(
    report: &PreflightReport,
    invocation: DirectPytestInvocation,
) -> Option<(PathBuf, Vec<OsString>, Vec<String>)> {
    let row = report
        .languages
        .iter()
        .find(|row| row.language == CourseLanguage::Python);

    match invocation {
        DirectPytestInvocation::UvRunPytest(extra) => {
            let uv = row
                .and_then(|row| row.managed.as_ref())
                .and_then(|managed| managed.uv.as_ref())
                .filter(|path| path.is_file())
                .cloned()
                .or_else(|| find_host_executable("uv"))?;
            Some((
                uv,
                vec![OsString::from("run"), OsString::from("pytest")],
                extra,
            ))
        }
        DirectPytestInvocation::Pytest(extra) => {
            if let Some(row) = row {
                match row.selected {
                    Some(RuntimeSource::Managed) => {
                        if let Some(managed) = &row.managed
                            && managed.executable.is_file()
                        {
                            return Some((
                                managed.executable.clone(),
                                vec![OsString::from("-m"), OsString::from("pytest")],
                                extra,
                            ));
                        }
                    }
                    Some(RuntimeSource::System { tool }) => {
                        if let Some(exe) = row
                            .system
                            .iter()
                            .find(|check| check.tool == tool)
                            .and_then(|check| check.executable.clone())
                            .filter(|path| path.is_file())
                        {
                            return Some((
                                exe,
                                vec![OsString::from("-m"), OsString::from("pytest")],
                                extra,
                            ));
                        }
                    }
                    None => {}
                }
            }
            if let Some(pytest) = find_host_executable("pytest") {
                return Some((pytest, Vec::new(), extra));
            }
            if let Some(python) =
                find_host_executable("python3").or_else(|| find_host_executable("python"))
            {
                return Some((
                    python,
                    vec![OsString::from("-m"), OsString::from("pytest")],
                    extra,
                ));
            }
            None
        }
    }
}

fn extract_python_error(raw_output: &str) -> Option<String> {
    let cleaned = strip_ansi_escapes::strip_str(raw_output);
    let markers = [
        "SyntaxError:",
        "IndentationError:",
        "ImportError:",
        "ModuleNotFoundError:",
        "ERROR collecting ",
    ];
    if markers.iter().any(|marker| cleaned.contains(marker)) {
        let excerpt: Vec<&str> = cleaned
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(15)
            .collect();
        if !excerpt.is_empty() {
            return Some(excerpt.join("\n"));
        }
    }
    None
}
