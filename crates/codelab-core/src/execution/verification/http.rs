use crate::{CheckDiff, CheckResult, CheckStatus, CourseCheckSpec, DomainError, ErrorCode};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) fn run(
    check: &CourseCheckSpec,
    cancelled: &AtomicBool,
) -> Result<CheckResult, DomainError> {
    let start = Instant::now();
    let invalid =
        || DomainError::new(ErrorCode::CourseChecksInvalid).with_detail("checkId", &check.id);
    let method = check
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse::<reqwest::Method>()
        .map_err(|_| invalid())?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(check.timeout_sec.unwrap_or(10)))
        .build()
        .map_err(|_| invalid())?;
    let url = check.url.as_deref().ok_or_else(invalid)?;
    let mut request = client.request(method, url);
    for (key, value) in &check.headers {
        request = request.header(key, value);
    }
    if let Some(body) = &check.body {
        request = request.body(body.clone());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(super::execution_error)?;
    let fetch = async {
        let mut response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            bytes.extend_from_slice(&chunk);
            if bytes.len() > 2 * 1024 * 1024 {
                break;
            }
        }
        Ok::<_, reqwest::Error>((status, headers, bytes))
    };
    let fetched = runtime.block_on(async {
        let mut pending = std::pin::pin!(fetch);
        loop {
            if cancelled.load(Ordering::Acquire) {
                return None;
            }
            if let Ok(result) =
                tokio::time::timeout(Duration::from_millis(20), pending.as_mut()).await
            {
                return Some(result);
            }
        }
    });
    let Some(fetched) = fetched else {
        return Err(DomainError::new(ErrorCode::ExecutionFailed));
    };
    let (response_status, headers, bytes) = match fetched {
        Ok(response) => response,
        Err(error) => {
            return Ok(CheckResult {
                check_id: check.id.clone(),
                status: if error.is_timeout() {
                    CheckStatus::TimedOut
                } else {
                    CheckStatus::Failed
                },
                duration_ms: start.elapsed().as_millis() as u64,
                diffs: vec![CheckDiff {
                    label: "response".into(),
                    expected: url.into(),
                    actual: String::new(),
                }],
                raw_output: String::new(),
                test_cases: Vec::new(),
                error: Some(
                    DomainError::new(ErrorCode::ExecutionFailed)
                        .with_detail("reason", error.to_string()),
                ),
            });
        }
    };
    let mut diffs = Vec::new();
    let mut compare = |label: &str, expected: String, actual: String, matches: bool| {
        if !matches {
            diffs.push(CheckDiff {
                label: label.into(),
                expected,
                actual,
            });
        }
    };
    let status = response_status.as_u16();
    let expected_status = check.expect.status.unwrap_or(200);
    compare(
        "status",
        expected_status.to_string(),
        status.to_string(),
        status == expected_status,
    );
    let mut raw_output = format!("HTTP {status}\n");
    for (name, value) in &headers {
        raw_output.push_str(&format!("{name}: {}\n", value.to_str().unwrap_or_default()));
    }
    for (name, expected) in &check.expect.headers {
        let actual = headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        compare(name, expected.clone(), actual.into(), expected == actual);
    }
    let body = String::from_utf8_lossy(&bytes).into_owned();
    if body.len() > 2 * 1024 * 1024 {
        return Err(invalid().with_detail("reasonCode", "RESPONSE_TOO_LARGE"));
    }
    raw_output.push('\n');
    raw_output.push_str(&body);
    if let Some(expected) = &check.expect.body_contains {
        compare(
            "bodyContains",
            expected.clone(),
            body.clone(),
            body.contains(expected),
        );
    }
    if let Some(expected) = &check.expect.body_equals {
        compare(
            "bodyEquals",
            expected.clone(),
            body.clone(),
            &body == expected,
        );
    }
    Ok(CheckResult {
        check_id: check.id.clone(),
        status: if diffs.is_empty() {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        duration_ms: start.elapsed().as_millis() as u64,
        diffs,
        raw_output,
        test_cases: Vec::new(),
        error: None,
    })
}
