use super::diff::extract_expected_actual;
use super::{go, java, js_ts, python};
use crate::execution::Directory;
use crate::runtime::PreflightReport;
use crate::{
    CheckDiff, CheckResult, CheckStatus, CourseCheckSpec, CourseLanguage, DomainError,
    TestCaseResult,
};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

type SuiteRunner = fn(SuiteRunContext<'_>) -> Result<CheckResult, DomainError>;

pub(super) struct TestSuiteAdapter {
    runner: SuiteRunner,
    pub default_command: &'static str,
    pub default_timeout_sec: u64,
}

pub(super) struct SuiteRunContext<'a> {
    pub check: &'a CourseCheckSpec,
    pub script: &'a str,
    pub explicit_command: bool,
    pub directory: Directory,
    pub report: &'a PreflightReport,
    pub timeout: Duration,
    pub cancelled: &'a AtomicBool,
}

impl TestSuiteAdapter {
    pub fn for_course(language: Option<CourseLanguage>) -> Option<Self> {
        match language {
            Some(CourseLanguage::Go) => Some(Self {
                runner: run_go,
                default_command: "go test -json -race ./...",
                default_timeout_sec: 120,
            }),
            Some(CourseLanguage::JsTs) => Some(Self {
                runner: js_ts::run,
                default_command: js_ts::DEFAULT_COMMAND,
                default_timeout_sec: js_ts::DEFAULT_TIMEOUT_SEC,
            }),
            Some(CourseLanguage::Python) => Some(Self {
                runner: python::run,
                default_command: python::DEFAULT_COMMAND,
                default_timeout_sec: python::DEFAULT_TIMEOUT_SEC,
            }),
            Some(CourseLanguage::Java) => Some(Self {
                runner: java::run,
                default_command: java::DEFAULT_COMMAND,
                default_timeout_sec: java::DEFAULT_TIMEOUT_SEC,
            }),
            None => None,
        }
    }

    pub fn run(&self, ctx: SuiteRunContext<'_>) -> Result<CheckResult, DomainError> {
        (self.runner)(ctx)
    }
}

fn run_go(ctx: SuiteRunContext<'_>) -> Result<CheckResult, DomainError> {
    let output = super::run_headless(
        ctx.script,
        ctx.directory,
        ctx.report,
        Some(CourseLanguage::Go),
        ctx.timeout,
        ctx.cancelled,
    )?;
    let mut result = super::command_result(ctx.check, output);
    if result.status == CheckStatus::TimedOut {
        return Ok(result);
    }
    let suite_report = go::parse_report(&result.raw_output);
    apply_suite_report(
        &mut result,
        suite_report,
        "goTestReport",
        &ctx.check.require_tests,
    );
    Ok(result)
}

pub(super) fn apply_suite_report(
    result: &mut CheckResult,
    report: SuiteReport,
    report_label: &str,
    required: &[String],
) {
    let has_failed_case = report.has_failed_case();
    if !report.passed() && (!report.completed_successfully || !has_failed_case) {
        let actual = report
            .compile_error
            .clone()
            .unwrap_or_else(|| result.raw_output.clone());
        result.diffs.push(CheckDiff {
            label: report_label.into(),
            expected: "pass".into(),
            actual,
        });
    }
    for name in required {
        if let Some(actual) = report.required_case_problem(name) {
            result.diffs.push(CheckDiff {
                label: name.clone(),
                expected: "pass".into(),
                actual: actual.into(),
            });
        }
    }
    result.test_cases = report.into_test_cases();
    for test in result
        .test_cases
        .iter()
        .filter(|test| test.status == CheckStatus::Failed)
    {
        if let Some(extracted) = extract_expected_actual(&test.output) {
            result.diffs.push(CheckDiff {
                label: test.name.clone(),
                expected: extracted.expected,
                actual: extracted.actual,
            });
        } else {
            result.diffs.push(CheckDiff {
                label: test.name.clone(),
                expected: "pass".into(),
                actual: test.output.clone(),
            });
        }
    }
    if result.status == CheckStatus::Passed && !result.diffs.is_empty() {
        result.status = CheckStatus::Failed;
    }
}

pub(super) struct SuiteReport {
    pub completed_successfully: bool,
    pub tests: BTreeMap<SuiteCaseId, TestCaseResult>,
    pub compile_error: Option<String>,
}

impl SuiteReport {
    pub fn incomplete() -> Self {
        Self {
            completed_successfully: false,
            tests: BTreeMap::new(),
            compile_error: None,
        }
    }

    pub fn passed(&self) -> bool {
        self.completed_successfully
            && self
                .tests
                .values()
                .all(|test| matches!(test.status, CheckStatus::Passed | CheckStatus::Skipped))
            && self
                .tests
                .values()
                .any(|test| test.status == CheckStatus::Passed)
    }

    pub fn has_failed_case(&self) -> bool {
        self.tests
            .values()
            .any(|test| test.status == CheckStatus::Failed)
    }

    fn required_case_problem(&self, name: &str) -> Option<&'static str> {
        if let Some((package, test)) = name.split_once("::")
            && let Some(case) = self.tests.get(&SuiteCaseId {
                package: package.to_owned(),
                test: test.to_owned(),
            })
        {
            return (case.status != CheckStatus::Passed).then_some("missingOrFailed");
        }
        let mut matches = self
            .tests
            .iter()
            .filter(|(id, _)| id.matches(name))
            .map(|(_, test)| test);
        let first = matches.next();
        if matches.next().is_some() {
            return Some("ambiguousTestName");
        }
        (!first.is_some_and(|test| test.status == CheckStatus::Passed)).then_some("missingOrFailed")
    }

    fn into_test_cases(self) -> Vec<TestCaseResult> {
        let mut counts = BTreeMap::new();
        for test in self.tests.values() {
            *counts.entry(test.name.clone()).or_insert(0usize) += 1;
        }
        let mut cases: Vec<_> = self
            .tests
            .into_iter()
            .map(|(id, mut case)| {
                if counts[&case.name] > 1 || id.test.contains("::") {
                    case.name = format!("{}::{}", id.package, id.test);
                }
                case
            })
            .collect();
        cases.sort_by(|left, right| left.name.cmp(&right.name));
        cases
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SuiteCaseId {
    pub package: String,
    pub test: String,
}

impl SuiteCaseId {
    pub fn joined_name(&self) -> String {
        format!("{}/{}", self.package, self.test)
    }

    fn matches(&self, selector: &str) -> bool {
        let normalized = selector.strip_suffix("()").unwrap_or(selector);
        self.test == selector
            || self.test == normalized
            || self.matches_qualified(selector, '/')
            || self.matches_qualified(normalized, '#')
            || self.matches_qualified(selector, '>')
    }

    fn matches_qualified(&self, selector: &str, separator: char) -> bool {
        let Some(rest) = selector.strip_prefix(&self.package) else {
            return false;
        };
        let rest = if separator == '>' {
            rest.strip_prefix(" > ")
        } else {
            rest.strip_prefix(separator)
        };
        rest == Some(self.test.as_str())
    }
}
