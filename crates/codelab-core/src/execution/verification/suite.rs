use super::go;
use crate::{CheckDiff, CheckResult, CheckStatus, CourseLanguage, TestCaseResult};
use std::collections::BTreeMap;

pub(super) struct TestSuiteAdapter {
    pub default_command: &'static str,
    pub default_timeout_sec: u64,
    report_label: &'static str,
    parse_report: fn(&str) -> SuiteReport,
}

impl TestSuiteAdapter {
    pub fn for_course(language: Option<CourseLanguage>) -> Option<Self> {
        match language {
            Some(CourseLanguage::Go) => Some(Self {
                default_command: "go test -json -race ./...",
                default_timeout_sec: 120,
                report_label: "goTestReport",
                parse_report: go::parse_report,
            }),
            _ => None,
        }
    }

    pub fn apply_report(&self, result: &mut CheckResult, required: &[String]) {
        let report = (self.parse_report)(&result.raw_output);
        if !report.passed() {
            result.diffs.push(CheckDiff {
                label: self.report_label.into(),
                expected: "pass".into(),
                actual: result.raw_output.clone(),
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
            result.diffs.push(CheckDiff {
                label: test.name.clone(),
                expected: "pass".into(),
                actual: test.output.clone(),
            });
        }
        if result.status == CheckStatus::Passed && !result.diffs.is_empty() {
            result.status = CheckStatus::Failed;
        }
    }
}

pub(super) struct SuiteReport {
    pub completed_successfully: bool,
    pub tests: BTreeMap<SuiteCaseId, TestCaseResult>,
}

impl SuiteReport {
    fn passed(&self) -> bool {
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
        self.test == selector
            || selector
                .strip_prefix(&self.package)
                .and_then(|rest| rest.strip_prefix('/'))
                == Some(self.test.as_str())
    }
}
