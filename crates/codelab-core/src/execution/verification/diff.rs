pub(super) struct ExtractedDiff {
    pub expected: String,
    pub actual: String,
}

pub(super) fn extract_expected_actual(raw_output: &str) -> Option<ExtractedDiff> {
    let cleaned = strip_ansi_escapes::strip_str(raw_output);
    extract_junit5(&cleaned)
        .or_else(|| extract_bun_scalar(&cleaned))
        .or_else(|| extract_unified_minus_expected_plus_actual(&cleaned))
        .or_else(|| extract_node_strict_not_equal(&cleaned))
        .or_else(|| extract_node_object_properties(&cleaned))
        .or_else(|| extract_pytest_assert_eq(&cleaned))
        .or_else(|| extract_inline_expected_actual(&cleaned))
}

fn extract_junit5(text: &str) -> Option<ExtractedDiff> {
    // JUnit 5: `[custom msg ==> ]expected: <EXPECTED> but was: <ACTUAL>`
    let marker = "expected: <";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let middle = "> but was: <";
    let mid = rest.find(middle)?;
    let expected = rest[..mid].trim();
    let after = &rest[mid + middle.len()..];
    let end = find_closing_angle(after)?;
    let actual = after[..end].trim();
    if expected.is_empty() && actual.is_empty() {
        return None;
    }
    Some(ExtractedDiff {
        expected: expected.to_owned(),
        actual: actual.to_owned(),
    })
}

fn find_closing_angle(text: &str) -> Option<usize> {
    // Prefer the `>` ending the first JUnit assertion line (before stack frames `\n\tat `).
    let limit = text
        .find("\n\tat ")
        .or_else(|| text.find("\n    at "))
        .unwrap_or(text.len());
    text[..limit].rfind('>')
}

fn extract_bun_scalar(text: &str) -> Option<ExtractedDiff> {
    // Bun:
    //   Expected: "world"
    //   Received: "hello"
    let mut expected = None;
    let mut actual = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Expected:") {
            let value = value.trim();
            if !value.is_empty() && expected.is_none() {
                expected = Some(value.to_owned());
            }
        } else if let Some(value) = trimmed.strip_prefix("Received:") {
            let value = value.trim();
            if !value.is_empty() && actual.is_none() {
                actual = Some(value.to_owned());
            }
        }
    }
    Some(ExtractedDiff {
        expected: expected?,
        actual: actual?,
    })
}

fn extract_unified_minus_expected_plus_actual(text: &str) -> Option<ExtractedDiff> {
    // Handles:
    // 1. Bun `toEqual` diff (`- Expected` / `+ Received`)
    // 2. Node `assert.deepStrictEqual` diff (`+ actual - expected`)
    // 3. Pytest `Full diff:` (`E         - ...` / `E         + ...`)
    let is_bun_diff = text.contains("- Expected") && text.contains("+ Received");
    let is_node_diff = text.contains("+ actual - expected");
    let is_pytest_diff = text.contains("Full diff:");
    if !is_bun_diff && !is_node_diff && !is_pytest_diff {
        return None;
    }

    let lines: Vec<&str> = text.lines().collect();
    let start = if is_node_diff {
        lines
            .iter()
            .position(|line| line.contains("+ actual - expected"))?
            + 1
    } else if is_pytest_diff {
        lines.iter().position(|line| line.contains("Full diff:"))? + 1
    } else {
        lines
            .iter()
            .position(|line| line.contains("expect(received)"))
            .map(|idx| idx + 1)
            .unwrap_or(0)
    };

    let pytest_col = if is_pytest_diff {
        lines
            .get(start.saturating_sub(1))
            .and_then(|line| line.find("Full diff:"))
    } else {
        None
    };

    let mut expected_lines = Vec::new();
    let mut actual_lines = Vec::new();
    let mut saw_minus = false;
    let mut saw_plus = false;

    for raw in &lines[start..] {
        let line = if let Some(col) = pytest_col {
            if let Some(after_e) = raw.strip_prefix('E') {
                let target = col.saturating_sub(1);
                let spaces = after_e
                    .bytes()
                    .take_while(|byte| *byte == b' ')
                    .count()
                    .min(target);
                &after_e[spaces..]
            } else {
                raw
            }
        } else {
            raw
        };
        let trimmed = line.trim();
        if trimmed.starts_with("- Expected")
            || trimmed.starts_with("+ Received")
            || trimmed.starts_with("at ")
            || trimmed.starts_with("Error [")
            || trimmed.starts_with("generatedMessage:")
            || trimmed.starts_with("code:")
        {
            if saw_minus || saw_plus {
                break;
            }
            continue;
        }
        if trimmed.is_empty() {
            if saw_minus || saw_plus {
                break;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ") {
            saw_minus = true;
            expected_lines.push(rest);
        } else if let Some(rest) = line.strip_prefix("+ ") {
            saw_plus = true;
            actual_lines.push(rest);
        } else if is_pytest_diff && line.starts_with("? ") {
            // Pytest difflib caret indicator line.
            continue;
        } else if let Some(rest) = line.strip_prefix("  ") {
            expected_lines.push(rest);
            actual_lines.push(rest);
        } else if saw_minus || saw_plus {
            break;
        }
    }

    if !saw_minus && !saw_plus {
        return None;
    }
    let expected = expected_lines.join("\n").trim().to_owned();
    let actual = actual_lines.join("\n").trim().to_owned();
    if expected.is_empty() && actual.is_empty() {
        return None;
    }
    Some(ExtractedDiff { expected, actual })
}

fn extract_node_strict_not_equal(text: &str) -> Option<ExtractedDiff> {
    // Node `assert.equal(actual, expected)`:
    // Expected values to be strictly equal:
    //
    // 1 !== 2
    let idx = text.find("Expected values to be strictly equal:")?;
    let after = &text[idx + "Expected values to be strictly equal:".len()..];
    for line in after.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((actual, expected)) = trimmed.split_once(" !== ") {
            return Some(ExtractedDiff {
                expected: expected.trim().to_owned(),
                actual: actual.trim().to_owned(),
            });
        }
        break;
    }
    None
}

fn extract_node_object_properties(text: &str) -> Option<ExtractedDiff> {
    // Node AssertionError properties block:
    //   actual: ...,
    //   expected: ...,
    let mut actual = None;
    let mut expected = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("actual:") {
            let value = rest.trim().trim_end_matches(',');
            if !value.is_empty() && actual.is_none() {
                actual = Some(value.to_owned());
            }
        } else if let Some(rest) = trimmed.strip_prefix("expected:") {
            let value = rest.trim().trim_end_matches(',');
            if !value.is_empty() && expected.is_none() {
                expected = Some(value.to_owned());
            }
        }
    }
    Some(ExtractedDiff {
        expected: expected?,
        actual: actual?,
    })
}

fn extract_pytest_assert_eq(text: &str) -> Option<ExtractedDiff> {
    // Pytest:
    // `AssertionError: assert <actual> == <expected>`
    // or `E       assert <actual> == <expected>`
    for line in text.lines() {
        let candidate = line.trim();
        let candidate = candidate
            .strip_prefix("E ")
            .or_else(|| candidate.strip_prefix("E\t"))
            .unwrap_or(candidate)
            .trim();
        let expr = candidate
            .strip_prefix("AssertionError: assert ")
            .or_else(|| candidate.strip_prefix("assert "));
        let Some(expr) = expr else {
            continue;
        };
        if let Some((actual, expected)) = split_top_level_eq(expr) {
            let actual = actual.trim();
            let expected = expected.trim();
            if !actual.is_empty() && !expected.is_empty() {
                return Some(ExtractedDiff {
                    expected: expected.to_owned(),
                    actual: actual.to_owned(),
                });
            }
        }
    }
    None
}

fn split_top_level_eq(expr: &str) -> Option<(&str, &str)> {
    let bytes = expr.as_bytes();
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut escape = false;
    let mut i = 0;
    while i + 1 < bytes.len() {
        let b = bytes[i];
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        if let Some(q) = quote {
            if b == b'\\' {
                escape = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'=' if depth == 0 && bytes[i + 1] == b'=' => {
                let prev = i.checked_sub(1).map(|idx| bytes[idx]);
                let next = bytes.get(i + 2).copied();
                if !matches!(prev, Some(b'!' | b'<' | b'>' | b'=')) && next != Some(b'=') {
                    return Some((&expr[..i], &expr[i + 2..]));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_inline_expected_actual(text: &str) -> Option<ExtractedDiff> {
    // Matches `expected: X actual: Y` or `expected: X, actual: Y`
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let Some(exp_idx) = lower.find("expected:") else {
            continue;
        };
        let after_exp_start = exp_idx + "expected:".len();
        let Some(act_rel) = lower[after_exp_start..].find("actual:") else {
            continue;
        };
        let expected = line[after_exp_start..after_exp_start + act_rel]
            .trim()
            .trim_end_matches([',', ';']);
        let actual = line[after_exp_start + act_rel + "actual:".len()..].trim();
        if !expected.is_empty() && !actual.is_empty() {
            return Some(ExtractedDiff {
                expected: expected.to_owned(),
                actual: actual.to_owned(),
            });
        }
    }
    None
}
