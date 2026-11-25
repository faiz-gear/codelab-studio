mod go;
mod http;
pub(crate) mod model;
mod suite;

use super::preparation::CommandInvocation;
use super::{Directory, Process, Spawn, environment, execution_error, shell_command};
use crate::runtime::PreflightReport;
use crate::{
    CheckDiff, CheckKind, CheckResult, CheckStatus, CourseCheckSpec, CourseLanguage, DomainError,
    ErrorCode,
};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use suite::TestSuiteAdapter;

const OUTPUT_LIMIT: u64 = 2 * 1024 * 1024;
const CHECK_SHELL: &str = "sh";

pub(crate) struct VerificationTask {
    pub chapter_id: Option<String>,
    pub cancelled: Arc<AtomicBool>,
    pub handle: Option<JoinHandle<()>>,
}
impl Drop for VerificationTask {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub(crate) struct PreparedCheck {
    spec: CourseCheckSpec,
    language: Option<CourseLanguage>,
    execution: CheckExecution,
}

enum CheckExecution {
    Http,
    Command {
        script: String,
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
                let script = check
                    .command
                    .as_deref()
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
                timeout,
                suite,
            } => {
                let output = run_headless(
                    &script,
                    directory,
                    report,
                    self.language,
                    timeout,
                    cancelled,
                )?;
                let mut result = command_result(&self.spec, output);
                if let Some(suite) = suite {
                    suite.apply_report(&mut result, &self.spec.require_tests);
                }
                Ok(result)
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
    let mut output = tempfile::tempfile().map_err(execution_error)?;
    let mut process = Process::spawn_headless(
        Spawn {
            program,
            args,
            directory,
            env: environment(report, language)?,
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
                let executable = word
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(word)
                    .to_ascii_lowercase();
                let executable = executable.strip_suffix(".exe").unwrap_or(&executable);
                if matches!(
                    executable,
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
                    executable,
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
