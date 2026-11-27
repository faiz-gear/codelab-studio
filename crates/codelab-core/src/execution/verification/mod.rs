mod diff;
mod go;
mod http;
mod java;
mod js_ts;
pub(crate) mod model;
mod python;
mod suite;
mod xml;

use super::preparation::CommandInvocation;
use super::{Directory, Process, Spawn, environment, execution_error, shell_command};
use crate::runtime::PreflightReport;
use crate::{
    CheckDiff, CheckKind, CheckResult, CheckStatus, CourseCheckSpec, CourseLanguage, DomainError,
    ErrorCode,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use suite::TestSuiteAdapter;

const OUTPUT_LIMIT: u64 = 2 * 1024 * 1024;
const CHECK_SHELL: &str = "sh";

pub(crate) struct PreparedCheck {
    spec: CourseCheckSpec,
    language: Option<CourseLanguage>,
    execution: CheckExecution,
}

enum CheckExecution {
    Http,
    Command {
        script: String,
        explicit_command: bool,
        timeout: Duration,
        suite: Option<TestSuiteAdapter>,
    },
}

impl PreparedCheck {
    pub fn prepare(
        check: &CourseCheckSpec,
        language: Option<CourseLanguage>,
    ) -> Result<Self, DomainError> {
        let execution = match check.kind {
            CheckKind::HttpAssert => CheckExecution::Http,
            CheckKind::CommandTest | CheckKind::TestSuite => {
                let suite = if check.kind == CheckKind::TestSuite {
                    Some(TestSuiteAdapter::for_course(language).ok_or_else(|| {
                        DomainError::new(ErrorCode::TestSuiteAdapterUnavailable)
                            .with_detail("checkId", &check.id)
                    })?)
                } else {
                    None
                };
                let explicit = check
                    .command
                    .as_deref()
                    .filter(|script| !script.trim().is_empty());
                let script = explicit
                    .or_else(|| suite.as_ref().map(|suite| suite.default_command))
                    .filter(|script| !script.trim().is_empty())
                    .ok_or_else(|| {
                        DomainError::new(ErrorCode::CourseChecksInvalid)
                            .with_detail("checkId", &check.id)
                    })?;
                let timeout = check.timeout_sec.unwrap_or_else(|| match &suite {
                    Some(suite) => suite.default_timeout_sec,
                    None => default_timeout_sec(language, script),
                });
                CheckExecution::Command {
                    script: script.to_owned(),
                    explicit_command: explicit.is_some(),
                    timeout: Duration::from_secs(timeout),
                    suite,
                }
            }
            CheckKind::SelfCheck => {
                return Err(DomainError::new(ErrorCode::CourseChecksInvalid)
                    .with_detail("checkId", &check.id));
            }
        };
        Ok(Self {
            spec: check.clone(),
            language,
            execution,
        })
    }

    pub fn invocation(&self, directory: &Directory) -> Option<CommandInvocation> {
        match &self.execution {
            CheckExecution::Http => None,
            CheckExecution::Command { script, .. } => {
                Some(CommandInvocation::new(script, &directory.path, CHECK_SHELL))
            }
        }
    }

    pub fn run(
        self,
        directory: Directory,
        report: &PreflightReport,
        cancelled: &AtomicBool,
    ) -> Result<CheckResult, DomainError> {
        match self.execution {
            CheckExecution::Http => http::run(&self.spec, cancelled),
            CheckExecution::Command {
                script,
                explicit_command,
                timeout,
                suite,
            } => {
                if let Some(suite) = suite {
                    suite.run(suite::SuiteRunContext {
                        check: &self.spec,
                        script: &script,
                        explicit_command,
                        directory,
                        report,
                        timeout,
                        cancelled,
                    })
                } else {
                    let output = run_headless(
                        &script,
                        directory,
                        report,
                        self.language,
                        timeout,
                        cancelled,
                    )?;
                    Ok(command_result(&self.spec, output))
                }
            }
        }
    }
}

struct CommandOutput {
    exit: u32,
    timed_out: bool,
    too_large: bool,
    duration_ms: u64,
    raw_output: String,
}

fn run_headless(
    script: &str,
    directory: Directory,
    report: &PreflightReport,
    language: Option<CourseLanguage>,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<CommandOutput, DomainError> {
    let (program, args) = shell_command(CHECK_SHELL, script)?;
    let env = environment(report, language)?;
    run_headless_spawn(program, args, directory, env, timeout, cancelled)
}

fn run_headless_spawn(
    program: PathBuf,
    args: Vec<OsString>,
    directory: Directory,
    env: BTreeMap<OsString, OsString>,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<CommandOutput, DomainError> {
    let mut output = tempfile::tempfile().map_err(execution_error)?;
    let mut process = Process::spawn_headless(
        Spawn {
            program,
            args,
            directory,
            env,
        },
        output.try_clone().map_err(execution_error)?,
    )
    .map_err(execution_error)?;
    let start = Instant::now();
    let (exit, timed_out, too_large) = loop {
        if let Some(exit) = process.reap_if_exited() {
            break (exit, false, false);
        }
        let timed_out = start.elapsed() >= timeout;
        let too_large = output.metadata().map_err(execution_error)?.len() > OUTPUT_LIMIT;
        if timed_out || too_large || cancelled.load(Ordering::Acquire) {
            break (process.interrupt(), timed_out, too_large);
        }
        thread::sleep(Duration::from_millis(10));
    };
    let too_large = too_large || output.metadata().map_err(execution_error)?.len() > OUTPUT_LIMIT;
    output.seek(SeekFrom::Start(0)).map_err(execution_error)?;
    let mut bytes = Vec::new();
    output
        .take(OUTPUT_LIMIT)
        .read_to_end(&mut bytes)
        .map_err(execution_error)?;
    Ok(CommandOutput {
        exit,
        timed_out,
        too_large,
        duration_ms: start.elapsed().as_millis() as u64,
        raw_output: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

fn read_bounded_file(path: &Path, limit: u64) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_simple_command(script: &str) -> Option<Vec<String>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(script, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }
    let mut cursor = root.walk();
    let mut commands = root
        .named_children(&mut cursor)
        .filter(|node| node.kind() != "comment");
    let node = commands.next()?;
    if commands.next().is_some() || node.kind() != "command" {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    let mut words = Vec::new();
    let mut child_cursor = node.walk();
    for child in node.named_children(&mut child_cursor) {
        let raw = &script[child.byte_range()];
        if child.id() == name.id() {
            if raw.contains(['$', '`', '*', '?', '[', '~', '"', '\'', '\\']) {
                return None;
            }
            words.push(raw.to_owned());
            continue;
        }
        if child.kind() != "word" && child.kind() != "raw_string" && child.kind() != "string" {
            return None;
        }
        if child.kind() == "string" && (raw.contains('$') || raw.contains('`')) {
            return None;
        }
        if child.kind() == "word" && raw.contains(['$', '`', '*', '?', '[', '~']) {
            return None;
        }
        let unquoted = match child.kind() {
            "raw_string" => raw.strip_prefix('\'')?.strip_suffix('\'')?.to_owned(),
            "string" => raw.strip_prefix('"')?.strip_suffix('"')?.to_owned(),
            _ => raw.to_owned(),
        };
        words.push(unquoted);
    }
    (!words.is_empty()).then_some(words)
}

fn command_result(check: &CourseCheckSpec, output: CommandOutput) -> CheckResult {
    let mut diffs = Vec::new();
    let expected = check.expect_exit_code.unwrap_or(0);
    if output.exit != expected {
        diffs.push(CheckDiff {
            label: "exitCode".into(),
            expected: expected.to_string(),
            actual: output.exit.to_string(),
        });
    }
    for text in &check.forbid_output {
        if output.raw_output.contains(text) {
            diffs.push(CheckDiff {
                label: "forbidOutput".into(),
                expected: String::new(),
                actual: text.clone(),
            });
        }
    }
    if output.too_large {
        diffs.push(CheckDiff {
            label: "outputLimit".into(),
            expected: OUTPUT_LIMIT.to_string(),
            actual: format!(">{OUTPUT_LIMIT}"),
        });
    }
    CheckResult {
        check_id: check.id.clone(),
        status: if output.timed_out {
            CheckStatus::TimedOut
        } else if diffs.is_empty() {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        duration_ms: output.duration_ms,
        diffs,
        raw_output: output.raw_output,
        test_cases: Vec::new(),
        error: None,
    }
}

fn default_timeout_sec(language: Option<CourseLanguage>, script: &str) -> u64 {
    if matches!(language, Some(CourseLanguage::Go | CourseLanguage::Java))
        || runs_compiled_toolchain_command(script)
    {
        120
    } else {
        30
    }
}

fn normalize_executable_name(raw: &str) -> String {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .to_ascii_lowercase();
    base.strip_suffix(".exe").unwrap_or(&base).to_owned()
}

fn runs_compiled_toolchain_command(script: &str) -> bool {
    let mut parser = tree_sitter::Parser::new();
    if parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .is_err()
    {
        return false;
    }
    let Some(tree) = parser.parse(script, None) else {
        return false;
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    loop {
        if cursor.node().kind() == "command"
            && let Some(name) = cursor.node().child_by_field_name("name")
        {
            let mut arg_cursor = cursor.node().walk();
            let mut words = std::iter::once(&script[name.byte_range()])
                .chain(
                    cursor
                        .node()
                        .children_by_field_name("argument", &mut arg_cursor)
                        .map(|node| &script[node.byte_range()]),
                )
                .peekable();
            while let Some(raw) = words.next() {
                let word = raw.trim_matches(|ch| ch == '\'' || ch == '"');
                let executable = normalize_executable_name(word);
                if matches!(
                    executable.as_str(),
                    "command" | "exec" | "env" | "nohup" | "builtin" | "time"
                ) {
                    while words
                        .peek()
                        .is_some_and(|next| next.starts_with('-') || next.contains('='))
                    {
                        words.next();
                    }
                    continue;
                }
                if matches!(
                    executable.as_str(),
                    "go" | "java" | "javac" | "mvn" | "mvnw" | "gradle" | "gradlew"
                ) {
                    return true;
                }
                break;
            }
        }
        if cursor.goto_first_child() || cursor.goto_next_sibling() {
            continue;
        }
        while cursor.goto_parent() {
            if cursor.goto_next_sibling() {
                break;
            }
        }
        if cursor.node() == root {
            return false;
        }
    }
}
