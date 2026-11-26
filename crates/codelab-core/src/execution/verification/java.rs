use super::suite::{SuiteRunContext, apply_suite_report};
use super::xml::{JunitXmlOptions, parse_junit_xml, parse_junit_xml_documents};
use super::{
    OUTPUT_LIMIT, command_result, parse_simple_command, read_bounded_file, run_headless,
    run_headless_spawn,
};
use crate::execution::environment;
use crate::runtime::{
    HostTool, PreflightReport, RuntimeSource, executable_name, find_host_executable,
};
use crate::{CheckDiff, CheckResult, CheckStatus, CourseLanguage, DomainError, ErrorCode};
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const XML_OPTIONS: JunitXmlOptions = JunitXmlOptions {
    strip_empty_parens: true,
    display_separator: "#",
};

pub(super) const DEFAULT_COMMAND: &str = "junit --scan-class-path";
pub(super) const DEFAULT_TIMEOUT_SEC: u64 = 12;
pub(super) const REPORT_LABEL: &str = "junitReport";

struct JavaToolchain {
    javac: PathBuf,
    java: PathBuf,
    junit_jar: PathBuf,
}

pub(super) fn run(ctx: SuiteRunContext<'_>) -> Result<CheckResult, DomainError> {
    let selector_args = if !ctx.explicit_command {
        Some(vec!["--scan-class-path".to_owned()])
    } else {
        classify_direct_junit_args(ctx.script)
    };

    if let Some(selector_args) = selector_args {
        return run_direct(ctx, selector_args);
    }

    let output = run_headless(
        ctx.script,
        ctx.directory,
        ctx.report,
        Some(CourseLanguage::Java),
        ctx.timeout,
        ctx.cancelled,
    )?;
    let mut result = command_result(ctx.check, output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }
    let suite_report = parse_junit_xml(&result.raw_output, XML_OPTIONS);
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
    selector_args: Vec<String>,
) -> Result<CheckResult, DomainError> {
    let start = Instant::now();
    let toolchain = resolve_java_toolchain(ctx.report, &ctx.directory.handle, &ctx.directory.path)
        .map_err(|code| DomainError::new(code).with_detail("checkId", &ctx.check.id))?;

    let mut sources = Vec::new();
    collect_java_sources(&ctx.directory.handle, Path::new(""), &mut sources)
        .map_err(super::execution_error)?;
    sources.sort();
    if sources.is_empty() {
        return Err(
            DomainError::new(ErrorCode::JavaSourcesNotFound).with_detail("checkId", &ctx.check.id)
        );
    }

    let temp = tempfile::tempdir().map_err(super::execution_error)?;
    let classes_dir = temp.path().join("classes");
    let reports_dir = temp.path().join("reports");
    let sources_file = temp.path().join("sources.txt");
    fs::create_dir_all(&classes_dir).map_err(super::execution_error)?;
    fs::create_dir_all(&reports_dir).map_err(super::execution_error)?;
    {
        let mut list = fs::File::create(&sources_file).map_err(super::execution_error)?;
        for source in &sources {
            writeln!(list, "{}", quote_javac_arg(source)).map_err(super::execution_error)?;
        }
    }

    let env = environment(ctx.report, Some(CourseLanguage::Java))?;
    let compile_dir = crate::execution::Directory {
        path: ctx.directory.path.clone(),
        handle: ctx
            .directory
            .handle
            .try_clone()
            .map_err(super::execution_error)?,
    };
    let javac_args = vec![
        OsString::from("-encoding"),
        OsString::from("UTF-8"),
        OsString::from("-cp"),
        toolchain.junit_jar.as_os_str().to_owned(),
        OsString::from("-d"),
        classes_dir.as_os_str().to_owned(),
        OsString::from(format!("@{}", sources_file.display())),
    ];
    let compile_output = run_headless_spawn(
        toolchain.javac,
        javac_args,
        compile_dir,
        env.clone(),
        ctx.timeout,
        ctx.cancelled,
    )?;

    if compile_output.timed_out {
        return Ok(command_result(ctx.check, compile_output));
    }
    if compile_output.exit != 0 {
        let compile_text = strip_ansi_escapes::strip_str(&compile_output.raw_output);
        let mut result = command_result(ctx.check, compile_output);
        let detail = compile_text.trim();
        if !detail.is_empty() {
            result.diffs.push(CheckDiff {
                label: REPORT_LABEL.into(),
                expected: "pass".into(),
                actual: detail.to_owned(),
            });
        }
        result.status = CheckStatus::Failed;
        return Ok(result);
    }

    let remaining = ctx.timeout.saturating_sub(start.elapsed());
    if remaining.is_zero() {
        return Ok(CheckResult {
            check_id: ctx.check.id.clone(),
            status: CheckStatus::TimedOut,
            duration_ms: start.elapsed().as_millis() as u64,
            diffs: Vec::new(),
            raw_output: compile_output.raw_output,
            test_cases: Vec::new(),
            error: None,
        });
    }

    let mut java_args = vec![
        OsString::from("-jar"),
        toolchain.junit_jar.as_os_str().to_owned(),
        OsString::from("execute"),
        OsString::from("--class-path"),
        classes_dir.as_os_str().to_owned(),
    ];
    if selector_args.is_empty() {
        java_args.push(OsString::from("--scan-class-path"));
    } else {
        java_args.extend(selector_args.into_iter().map(OsString::from));
    }
    java_args.push(OsString::from("--reports-dir"));
    java_args.push(reports_dir.as_os_str().to_owned());
    java_args.push(OsString::from("--details=tree"));
    java_args.push(OsString::from("--disable-ansi-colors"));

    let mut run_output = run_headless_spawn(
        toolchain.java,
        java_args,
        ctx.directory,
        env,
        remaining,
        ctx.cancelled,
    )?;
    if !compile_output.raw_output.trim().is_empty() {
        run_output.raw_output = format!(
            "{}\n{}",
            compile_output.raw_output.trim_end(),
            run_output.raw_output
        );
    }
    run_output.duration_ms = start.elapsed().as_millis() as u64;
    let mut result = command_result(ctx.check, run_output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }

    let xml_docs = read_junit_xml_reports(&reports_dir);
    let suite_report = if !xml_docs.is_empty() {
        parse_junit_xml_documents(&xml_docs, XML_OPTIONS)
    } else {
        parse_junit_xml(&result.raw_output, XML_OPTIONS)
    };
    apply_suite_report(
        &mut result,
        suite_report,
        REPORT_LABEL,
        &ctx.check.require_tests,
    );
    Ok(result)
}

fn classify_direct_junit_args(script: &str) -> Option<Vec<String>> {
    let words = parse_simple_command(script)?;
    if words.first().map(String::as_str) != Some("junit") {
        return None;
    }
    let rest = &words[1..];
    let mut idx = 0;
    while idx < rest.len() {
        let token = rest[idx].as_str();
        match token {
            "--scan-class-path" => idx += 1,
            "--select-class"
            | "--select-package"
            | "--select-method"
            | "--include-classname"
            | "--exclude-classname"
            | "-c"
            | "-p"
            | "-m"
            | "-n"
            | "-N" => {
                if idx + 1 >= rest.len() {
                    return None;
                }
                idx += 2;
            }
            value
                if value.starts_with("--select-class=")
                    || value.starts_with("--select-package=")
                    || value.starts_with("--select-method=")
                    || value.starts_with("--include-classname=")
                    || value.starts_with("--exclude-classname=") =>
            {
                idx += 1;
            }
            _ => return None,
        }
    }
    Some(rest.to_vec())
}

fn resolve_java_toolchain(
    report: &PreflightReport,
    workspace_handle: &Dir,
    workspace_dir: &Path,
) -> Result<JavaToolchain, ErrorCode> {
    let row = report
        .languages
        .iter()
        .find(|row| row.language == CourseLanguage::Java);

    let (javac, java) = if let Some(row) = row {
        match row.selected {
            Some(RuntimeSource::Managed) => {
                let managed = row
                    .managed
                    .as_ref()
                    .ok_or(ErrorCode::JavaToolchainUnavailable)?;
                let java = managed.executable.clone();
                let javac = java.with_file_name(executable_name("javac"));
                (javac, java)
            }
            Some(RuntimeSource::System {
                tool: HostTool::Javac,
            }) => {
                let javac = row
                    .system
                    .iter()
                    .find(|check| check.tool == HostTool::Javac)
                    .and_then(|check| check.executable.clone())
                    .or_else(find_host_javac)
                    .ok_or(ErrorCode::JavaToolchainUnavailable)?;
                let java = javac.with_file_name(executable_name("java"));
                (javac, java)
            }
            _ => {
                let javac = find_host_javac().ok_or(ErrorCode::JavaToolchainUnavailable)?;
                let java = javac.with_file_name(executable_name("java"));
                (javac, java)
            }
        }
    } else {
        let javac = find_host_javac().ok_or(ErrorCode::JavaToolchainUnavailable)?;
        let java = javac.with_file_name(executable_name("java"));
        (javac, java)
    };

    if !javac.is_file() || !java.is_file() {
        return Err(ErrorCode::JavaToolchainUnavailable);
    }

    let junit_jar = find_junit_jar(row, report, workspace_handle, workspace_dir)
        .ok_or(ErrorCode::JavaJunitLauncherUnavailable)?;

    Ok(JavaToolchain {
        javac,
        java,
        junit_jar,
    })
}

fn find_host_javac() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let candidate = PathBuf::from(home)
            .join("bin")
            .join(executable_name("javac"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    find_host_executable("javac")
}

fn is_junit_standalone_jar(name: &str) -> bool {
    name.starts_with("junit-platform-console-standalone") && name.ends_with(".jar")
}

fn find_junit_jar(
    row: Option<&crate::runtime::LanguagePreflight>,
    report: &PreflightReport,
    workspace_handle: &Dir,
    workspace_dir: &Path,
) -> Option<PathBuf> {
    if let Some(managed) = row.and_then(|row| row.managed.as_ref())
        && managed.test_executor.is_file()
    {
        return Some(managed.test_executor.clone());
    }
    for var in ["CODELAB_JUNIT_JAR", "JUNIT_PLATFORM_CONSOLE_STANDALONE_JAR"] {
        if let Some(value) = std::env::var_os(var) {
            let path = PathBuf::from(value);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    if let Ok(entries) = fs::read_dir(&report.install_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("java-")
                && let Ok(sub) = fs::read_dir(entry.path())
            {
                for item in sub.flatten() {
                    let fname = item.file_name();
                    if is_junit_standalone_jar(&fname.to_string_lossy()) && item.path().is_file() {
                        return Some(item.path());
                    }
                }
            }
        }
    }
    if let Ok(lib_dir) = workspace_handle.open_dir_nofollow("lib")
        && let Ok(entries) = lib_dir.entries()
    {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let Some(name_str) = fname.to_str() else {
                continue;
            };
            if is_junit_standalone_jar(name_str)
                && entry.file_type().is_ok_and(|kind| kind.is_file())
            {
                return Some(workspace_dir.join("lib").join(name_str));
            }
        }
    }
    None
}

fn collect_java_sources(dir: &Dir, prefix: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in dir.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if matches!(
            name_str,
            ".codelab" | ".git" | "target" | "build" | "out" | "node_modules"
        ) {
            continue;
        }
        let file_type = entry.file_type()?;
        let rel = prefix.join(name_str);
        if file_type.is_dir() {
            let child = dir.open_dir_nofollow(&name)?;
            collect_java_sources(&child, &rel, out)?;
        } else if file_type.is_file() && name_str.ends_with(".java") {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn quote_javac_arg(path: &str) -> String {
    if path.contains([' ', '\t', '"', '\'']) {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        path.to_owned()
    }
}

fn read_junit_xml_reports(reports_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(reports_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("TEST-") && name.ends_with(".xml"))
        })
        .collect();
    paths.sort();

    let mut remaining = OUTPUT_LIMIT;
    let mut docs = Vec::new();
    for path in paths {
        if remaining == 0 {
            break;
        }
        if let Ok(content) = read_bounded_file(&path, remaining) {
            remaining = remaining.saturating_sub(content.len() as u64);
            if !content.trim().is_empty() {
                docs.push(content);
            }
        }
    }
    docs
}
