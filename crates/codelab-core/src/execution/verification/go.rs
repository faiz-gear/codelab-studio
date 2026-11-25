use crate::{CheckDiff, CheckResult, CheckStatus, TestCaseResult};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GoEvent {
    action: String,
    #[serde(default)]
    package: String,
    test: Option<String>,
    output: Option<String>,
    elapsed: Option<f64>,
}

pub(super) fn adapt(result: &mut CheckResult, required: &[String]) {
    let mut tests = BTreeMap::<String, TestCaseResult>::new();
    let mut package_finished = false;
    for event in result
        .raw_output
        .lines()
        .filter_map(|line| serde_json::from_str::<GoEvent>(line).ok())
    {
        let Some(test) = event.test else {
            if matches!(event.action.as_str(), "pass" | "skip") {
                package_finished = true;
            }
            continue;
        };
        let name = format!("{}/{test}", event.package);
        let case = tests.entry(name.clone()).or_insert_with(|| TestCaseResult {
            name,
            status: CheckStatus::Running,
            duration_ms: 0,
            output: String::new(),
        });
        if let Some(output) = event.output {
            case.output.push_str(&output);
        }
        match event.action.as_str() {
            "pass" => case.status = CheckStatus::Passed,
            "skip" => case.status = CheckStatus::Skipped,
            "fail" => case.status = CheckStatus::Failed,
            _ => {}
        }
        if let Some(seconds) = event.elapsed {
            case.duration_ms = (seconds * 1000.0) as u64;
        }
    }
    for test in tests.values().filter(|t| t.status == CheckStatus::Failed) {
        result.diffs.push(CheckDiff {
            label: test.name.clone(),
            expected: "pass".into(),
            actual: test.output.clone(),
        });
    }
    if result.status == CheckStatus::Passed
        && (!package_finished
            || tests
                .values()
                .any(|t| matches!(t.status, CheckStatus::Running | CheckStatus::Failed)))
    {
        result.status = CheckStatus::Failed;
        result.diffs.push(CheckDiff {
            label: "goTestReport".into(),
            expected: "pass".into(),
            actual: result.raw_output.clone(),
        });
    }
    for name in required {
        if !tests
            .values()
            .find(|test| test.name == *name || test.name.ends_with(&format!("/{name}")))
            .is_some_and(|test| test.status == CheckStatus::Passed)
        {
            result.status = CheckStatus::Failed;
            result.diffs.push(CheckDiff {
                label: name.clone(),
                expected: "pass".into(),
                actual: "missingOrFailed".into(),
            });
        }
    }
    result.test_cases = tests.into_values().collect();
}
