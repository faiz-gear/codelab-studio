mod go;
mod http;
pub(crate) mod model;
use super::{CommandLaunch, Directory, Process, Spawn, environment, execution_error, shell};
use crate::runtime::PreflightReport;
use crate::{CheckDiff, CheckResult, CheckStatus, CourseCheckSpec, CourseLanguage, DomainError};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
pub(crate) fn command_block(
    check: &CourseCheckSpec,
    language: Option<CourseLanguage>,
) -> crate::CommandBlock {
    crate::CommandBlock {
        id: check.id.clone(),
        shell: "sh".into(),
        command: check.command.clone().unwrap_or_else(|| match language {
            Some(CourseLanguage::Go) => "go test -json -race ./...".into(),
            _ => String::new(),
        }),
        cwd: check.cwd.clone(),
        cwd_origin: crate::ValueOrigin::Explicit,
        mode: crate::CommandMode::Oneshot,
        mode_origin: crate::ValueOrigin::Explicit,
        label: Some(check.title.clone()),
        port: None,
        is_secondary: false,
    }
}
pub(crate) fn run_command(
    check: &CourseCheckSpec,
    directory: Directory,
    report: &PreflightReport,
    language: Option<CourseLanguage>,
    cancelled: &AtomicBool,
) -> Result<CheckResult, DomainError> {
    if check.kind == crate::CheckKind::HttpAssert {
        return http::run(check, cancelled);
    }
    let block = command_block(check, language);
    if block.command.trim().is_empty() {
        return Err(DomainError::new(crate::ErrorCode::CourseChecksInvalid)
            .with_detail("checkId", &check.id));
    }
    let launch = CommandLaunch {
        script: block.command.clone(),
        cwd: directory.path.clone(),
        block,
        restart_terminal_id: None,
        cwd_key: String::new(),
    };
    let (program, args) = shell(Some(&launch))?;
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
    let timeout = check
        .timeout_sec
        .unwrap_or_else(|| default_timeout_sec(language, &launch.script));
    let (exit, timed_out, too_large) = loop {
        if let Some(exit) = process.reap_if_exited() {
            break (exit, false, false);
        }
        let timed_out = start.elapsed() >= Duration::from_secs(timeout);
        let too_large = output.metadata().map_err(execution_error)?.len() > 2 * 1024 * 1024;
        if timed_out || too_large || cancelled.load(Ordering::Acquire) {
            break (process.interrupt(), timed_out, too_large);
        }
        thread::sleep(Duration::from_millis(10));
    };
    output.seek(SeekFrom::Start(0)).map_err(execution_error)?;
    let mut bytes = Vec::new();
    output
        .take(2 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(execution_error)?;
    let raw_output = String::from_utf8_lossy(&bytes).into_owned();
    let mut diffs = Vec::new();
    let expected = check.expect_exit_code.unwrap_or(0);
    if exit != expected {
        diffs.push(CheckDiff {
            label: "exitCode".into(),
            expected: expected.to_string(),
            actual: exit.to_string(),
        });
    }
    for text in &check.forbid_output {
        if raw_output.contains(text) {
            diffs.push(CheckDiff {
                label: "forbidOutput".into(),
                expected: String::new(),
                actual: text.clone(),
            });
        }
    }
    if too_large {
        diffs.push(CheckDiff {
            label: "outputLimit".into(),
            expected: "2097152".into(),
            actual: ">2097152".into(),
        });
    }
    let mut result = CheckResult {
        check_id: check.id.clone(),
        status: if timed_out {
            CheckStatus::TimedOut
        } else if diffs.is_empty() {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        duration_ms: start.elapsed().as_millis() as u64,
        diffs,
        raw_output,
        test_cases: Vec::new(),
        error: None,
    };
    if launch.script.contains("go test") && launch.script.contains("-json") {
        go::adapt(&mut result, &check.require_tests);
    }
    Ok(result)
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
