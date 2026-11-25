use super::suite::{SuiteCaseId, SuiteReport};
use crate::{CheckStatus, TestCaseResult};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GoEvent {
    action: Option<String>,
    import_path: Option<String>,
    package: Option<String>,
    test: Option<String>,
    output: Option<String>,
    elapsed: Option<f64>,
}

impl GoEvent {
    fn is_test_action(&self) -> bool {
        matches!(
            self.action.as_deref(),
            Some(
                "start"
                    | "run"
                    | "pause"
                    | "cont"
                    | "pass"
                    | "skip"
                    | "fail"
                    | "bench"
                    | "output"
                    | "attr"
                    | "artifacts"
                    | "build-output"
                    | "build-fail"
            )
        )
    }
}

pub(super) fn parse_report(output: &str) -> SuiteReport {
    let mut tests = BTreeMap::<SuiteCaseId, TestCaseResult>::new();
    let mut packages = BTreeMap::<String, BTreeSet<String>>::new();
    let mut complete = true;
    for line in output.lines() {
        if !line.trim_start().starts_with('{') {
            continue;
        }
        let event = match serde_json::from_str::<GoEvent>(line) {
            Ok(event) => event,
            Err(_) => {
                complete = false;
                continue;
            }
        };
        match event.action.as_deref() {
            Some("build-output") if event.import_path.is_some() => continue,
            Some("build-fail") if event.import_path.is_some() => {
                complete = false;
                continue;
            }
            _ if event.package.is_none()
                && event.import_path.is_none()
                && !event.is_test_action() =>
            {
                continue;
            }
            _ => {}
        }
        let Some(package) = event
            .package
            .filter(|name| !name.is_empty() && !name.contains(':'))
        else {
            complete = false;
            continue;
        };
        let action = event.action.as_deref().unwrap_or_default();
        if action == "start" && event.test.is_none() {
            complete &= packages.insert(package, BTreeSet::new()).is_none();
            continue;
        }
        let Some(active) = packages.get_mut(&package) else {
            complete = false;
            continue;
        };
        let benchmark = match action {
            "bench" => event.test.as_deref(),
            "output" => event.output.as_deref().and_then(benchmark_timing_name),
            _ => None,
        };
        // Successful benchmarks may emit a timing line in place of a test
        // terminal event. They do not supply a passing test case.
        if let Some(name) = benchmark
            && let Some(test) = active_benchmark(active, name)
        {
            mark_benchmark_complete(&mut tests, &package, &test);
            // A parent with subbenchmarks has no timing or terminal event.
            for (index, _) in test.match_indices('/') {
                let parent = &test[..index];
                if active.remove(parent) {
                    mark_benchmark_complete(&mut tests, &package, parent);
                }
            }
            if action == "bench" || event.test.is_none() {
                continue;
            }
        }
        let Some(test) = event.test else {
            match action {
                "pass" | "skip" | "fail" => {
                    complete &= active.is_empty() && action != "fail";
                    packages.remove(&package);
                }
                "output" | "attr" | "artifacts" => {}
                _ => complete = false,
            }
            continue;
        };
        complete &= !test.is_empty();
        let id = SuiteCaseId { package, test };
        let case = tests.entry(id.clone()).or_insert_with(|| TestCaseResult {
            name: id.joined_name(),
            status: CheckStatus::Running,
            duration_ms: 0,
            output: String::new(),
        });
        if let Some(output) = event.output {
            case.output.push_str(&output);
        }
        match action {
            "run" => complete &= active.insert(id.test),
            "pass" | "skip" | "fail" => {
                complete &= active.remove(&id.test);
                match action {
                    "pass" if case.status != CheckStatus::Failed => {
                        case.status = CheckStatus::Passed;
                    }
                    "skip" if case.status == CheckStatus::Running => {
                        case.status = CheckStatus::Skipped;
                    }
                    "fail" => case.status = CheckStatus::Failed,
                    _ => {}
                }
            }
            "bench" if id.test.starts_with("Benchmark") => {
                if case.status == CheckStatus::Running {
                    case.status = CheckStatus::Skipped;
                }
            }
            "pause" | "cont" => complete &= active.contains(&id.test),
            "output" | "attr" | "artifacts" => {}
            _ => complete = false,
        }
        if let Some(seconds) = event.elapsed {
            case.duration_ms = case.duration_ms.saturating_add((seconds * 1000.0) as u64);
        }
    }
    SuiteReport {
        completed_successfully: complete && packages.is_empty(),
        tests,
    }
}

fn benchmark_timing_name(output: &str) -> Option<&str> {
    let mut columns = output
        .trim_end()
        .split('\t')
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let name = columns.next()?.trim();
    let iterations = columns.next()?.parse::<u64>().ok()?;
    (name.starts_with("Benchmark") && iterations > 0 && columns.next()?.ends_with(" ns/op"))
        .then_some(name)
}

fn active_benchmark(active: &mut BTreeSet<String>, name: &str) -> Option<String> {
    if !name.starts_with("Benchmark") {
        return None;
    }
    if let Some(exact) = active.take(name) {
        return Some(exact);
    }
    let (test, cpu_count) = name.rsplit_once('-')?;
    cpu_count.parse::<u32>().ok()?;
    active.take(test)
}

fn mark_benchmark_complete(
    tests: &mut BTreeMap<SuiteCaseId, TestCaseResult>,
    package: &str,
    test: &str,
) {
    let id = SuiteCaseId {
        package: package.to_owned(),
        test: test.to_owned(),
    };
    if let Some(case) = tests.get_mut(&id)
        && case.status == CheckStatus::Running
    {
        case.status = CheckStatus::Skipped;
    }
}
