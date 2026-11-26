use super::suite::{SuiteCaseId, SuiteReport};
use crate::{CheckStatus, TestCaseResult};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct JunitXmlOptions {
    pub strip_empty_parens: bool,
    pub display_separator: &'static str,
}

#[derive(Debug)]
struct Element {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Element>,
    text: String,
}

pub(super) fn parse_junit_xml(input: &str, options: JunitXmlOptions) -> SuiteReport {
    let mut tests = BTreeMap::new();
    let trimmed = input.trim_start_matches('\u{feff}').trim();
    let Some(xml) = locate_xml_start(trimmed) else {
        return SuiteReport::incomplete();
    };
    let Ok(roots) = parse_documents(xml) else {
        return SuiteReport::incomplete();
    };
    if roots.is_empty() {
        return SuiteReport::incomplete();
    }
    let mut complete = true;
    let mut saw_suite_root = false;
    for root in &roots {
        match root.name.as_str() {
            "testsuites" | "testsuite" => {
                saw_suite_root = true;
                collect_element(root, "", options, &mut tests, &mut complete);
            }
            _ => complete = false,
        }
    }
    SuiteReport {
        completed_successfully: saw_suite_root && complete,
        tests,
        compile_error: None,
    }
}

pub(super) fn parse_junit_xml_documents(
    documents: &[String],
    options: JunitXmlOptions,
) -> SuiteReport {
    if documents.is_empty() {
        return SuiteReport::incomplete();
    }
    let mut combined = SuiteReport {
        completed_successfully: true,
        tests: BTreeMap::new(),
        compile_error: None,
    };
    for doc in documents {
        let part = parse_junit_xml(doc, options);
        combined.completed_successfully &= part.completed_successfully;
        for (id, case) in part.tests {
            combined.tests.insert(id, case);
        }
    }
    combined
}

fn locate_xml_start(input: &str) -> Option<&str> {
    if input.starts_with("<?xml")
        || input.starts_with("<testsuites")
        || input.starts_with("<testsuite")
    {
        return Some(input);
    }
    for marker in ["<?xml", "<testsuites", "<testsuite"] {
        if let Some(index) = input.find(marker) {
            return Some(&input[index..]);
        }
    }
    None
}

fn collect_element(
    element: &Element,
    enclosing_suite: &str,
    options: JunitXmlOptions,
    tests: &mut BTreeMap<SuiteCaseId, TestCaseResult>,
    complete: &mut bool,
) {
    match element.name.as_str() {
        "testsuites" => {
            for child in &element.children {
                match child.name.as_str() {
                    "testsuite" | "testcase" => {
                        collect_element(child, enclosing_suite, options, tests, complete);
                    }
                    _ => {}
                }
            }
        }
        "testsuite" => {
            let suite_name = element
                .attrs
                .get("name")
                .map(String::as_str)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(enclosing_suite);
            let before_failed = count_failed(tests);
            let before_total = tests.len();
            for child in &element.children {
                match child.name.as_str() {
                    "testsuite" | "testcase" => {
                        collect_element(child, suite_name, options, tests, complete);
                    }
                    "failure" | "error" => {
                        *complete = false;
                    }
                    _ => {}
                }
            }
            let reported_failures = parse_count(&element.attrs, "failures");
            let reported_errors = parse_count(&element.attrs, "errors");
            let added_failed = count_failed(tests).saturating_sub(before_failed);
            if reported_failures.saturating_add(reported_errors) > 0 && added_failed == 0 {
                *complete = false;
            }
            let reported_tests = parse_count(&element.attrs, "tests");
            let added_total = tests.len().saturating_sub(before_total);
            if reported_tests > 0 && added_total == 0 {
                *complete = false;
            }
        }
        "testcase" => {
            let Some(raw_name) = element
                .attrs
                .get("name")
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
            else {
                *complete = false;
                return;
            };
            let test_name = if options.strip_empty_parens {
                raw_name.strip_suffix("()").unwrap_or(raw_name)
            } else {
                raw_name
            };
            let package = element
                .attrs
                .get("classname")
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .or_else(|| (!enclosing_suite.is_empty()).then_some(enclosing_suite))
                .or_else(|| {
                    element
                        .attrs
                        .get("file")
                        .map(|name| name.trim())
                        .filter(|name| !name.is_empty())
                })
                .unwrap_or_default()
                .to_owned();
            let duration_ms = element
                .attrs
                .get("time")
                .and_then(|value| value.trim().parse::<f64>().ok())
                .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
                .map(|seconds| (seconds * 1000.0).round() as u64)
                .unwrap_or(0);

            let mut status = CheckStatus::Passed;
            let mut outputs = Vec::new();
            let mut has_failure_child = false;
            let mut has_skipped_child = false;

            for child in &element.children {
                match child.name.as_str() {
                    "failure" | "error" => {
                        has_failure_child = true;
                        status = CheckStatus::Failed;
                        push_detail(&mut outputs, child);
                    }
                    "skipped" => {
                        has_skipped_child = true;
                        if !has_failure_child {
                            status = CheckStatus::Skipped;
                        }
                        push_detail(&mut outputs, child);
                    }
                    "system-out" | "system-err" => {
                        let text = child.text.trim();
                        if !text.is_empty() {
                            outputs.push(text.to_owned());
                        }
                    }
                    _ => {}
                }
            }

            if !has_failure_child
                && !has_skipped_child
                && let Some(failure_attr) = element
                    .attrs
                    .get("failure")
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
            {
                status = CheckStatus::Failed;
                outputs.push(failure_attr.to_owned());
            }

            let display_name = if package.is_empty() {
                test_name.to_owned()
            } else {
                format!("{package}{}{test_name}", options.display_separator)
            };
            let id = SuiteCaseId {
                package,
                test: test_name.to_owned(),
            };
            let output = strip_ansi_escapes::strip_str(outputs.join("\n"));
            let entry = tests.entry(id).or_insert_with(|| TestCaseResult {
                name: display_name,
                status,
                duration_ms,
                output: output.clone(),
            });
            if entry.status != CheckStatus::Failed && status == CheckStatus::Failed {
                entry.status = CheckStatus::Failed;
                entry.output = output;
            }
        }
        _ => {}
    }
}

fn push_detail(outputs: &mut Vec<String>, child: &Element) {
    let message = child
        .attrs
        .get("message")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let body = child.text.trim();
    match (message, body.is_empty()) {
        (Some(msg), true) => outputs.push(msg.to_owned()),
        (None, false) => outputs.push(body.to_owned()),
        (Some(msg), false) => {
            if body.contains(msg) {
                outputs.push(body.to_owned());
            } else {
                outputs.push(format!("{msg}\n{body}"));
            }
        }
        (None, true) => {
            if let Some(kind) = child
                .attrs
                .get("type")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                outputs.push(kind.to_owned());
            }
        }
    }
}

fn parse_count(attrs: &BTreeMap<String, String>, key: &str) -> usize {
    attrs
        .get(key)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn count_failed(tests: &BTreeMap<SuiteCaseId, TestCaseResult>) -> usize {
    tests
        .values()
        .filter(|test| test.status == CheckStatus::Failed)
        .count()
}

struct XmlParser<'a> {
    input: &'a str,
    pos: usize,
}

fn parse_documents(input: &str) -> Result<Vec<Element>, ()> {
    let mut parser = XmlParser { input, pos: 0 };
    let mut roots = Vec::new();
    loop {
        parser.skip_misc()?;
        if parser.is_eof() {
            break;
        }
        if !parser.rest().starts_with('<') {
            return Err(());
        }
        roots.push(parser.parse_element(0)?);
    }
    Ok(roots)
}

impl<'a> XmlParser<'a> {
    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn skip_whitespace(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.input.len() - trimmed.len();
    }

    fn skip_misc(&mut self) -> Result<(), ()> {
        loop {
            self.skip_whitespace();
            let rest = self.rest();
            if rest.starts_with("<!--") {
                let end = rest.find("-->").ok_or(())?;
                self.pos += end + 3;
            } else if rest.starts_with("<?") {
                let end = rest.find("?>").ok_or(())?;
                self.pos += end + 2;
            } else if rest.starts_with("<!DOCTYPE") {
                let end = rest.find('>').ok_or(())?;
                self.pos += end + 1;
            } else {
                return Ok(());
            }
        }
    }

    fn parse_element(&mut self, depth: usize) -> Result<Element, ()> {
        if depth > 64 {
            return Err(());
        }
        if !self.rest().starts_with('<')
            || self.rest().starts_with("</")
            || self.rest().starts_with("<!")
        {
            return Err(());
        }
        self.pos += 1;
        let name = self.parse_name()?;
        let mut attrs = BTreeMap::new();
        loop {
            self.skip_whitespace();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.pos += 2;
                return Ok(Element {
                    name,
                    attrs,
                    children: Vec::new(),
                    text: String::new(),
                });
            }
            if rest.starts_with('>') {
                self.pos += 1;
                break;
            }
            if rest.is_empty() {
                return Err(());
            }
            let attr_name = self.parse_name()?;
            self.skip_whitespace();
            if !self.rest().starts_with('=') {
                return Err(());
            }
            self.pos += 1;
            self.skip_whitespace();
            let value = self.parse_attr_value()?;
            attrs.insert(attr_name, value);
        }

        let mut children = Vec::new();
        let mut text = String::new();
        loop {
            if self.is_eof() {
                return Err(());
            }
            let rest = self.rest();
            if rest.starts_with("</") {
                self.pos += 2;
                let closing = self.parse_name()?;
                self.skip_whitespace();
                if closing != name || !self.rest().starts_with('>') {
                    return Err(());
                }
                self.pos += 1;
                return Ok(Element {
                    name,
                    attrs,
                    children,
                    text,
                });
            } else if rest.starts_with("<![CDATA[") {
                self.pos += 9;
                let end = self.rest().find("]]>").ok_or(())?;
                text.push_str(&self.rest()[..end]);
                self.pos += end + 3;
            } else if rest.starts_with("<!--") {
                let end = rest.find("-->").ok_or(())?;
                self.pos += end + 3;
            } else if rest.starts_with("<?") {
                let end = rest.find("?>").ok_or(())?;
                self.pos += end + 2;
            } else if rest.starts_with('<') {
                children.push(self.parse_element(depth + 1)?);
            } else {
                let next_tag = rest.find('<').unwrap_or(rest.len());
                text.push_str(&unescape_xml(&rest[..next_tag]));
                self.pos += next_tag;
            }
        }
    }

    fn parse_name(&mut self) -> Result<String, ()> {
        let rest = self.rest();
        let end = rest
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.')))
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(());
        }
        let name = rest[..end].to_owned();
        self.pos += end;
        Ok(name)
    }

    fn parse_attr_value(&mut self) -> Result<String, ()> {
        let quote = self.rest().chars().next().ok_or(())?;
        if quote != '"' && quote != '\'' {
            return Err(());
        }
        self.pos += 1;
        let end = self.rest().find(quote).ok_or(())?;
        let raw = &self.rest()[..end];
        self.pos += end + 1;
        Ok(unescape_xml(raw))
    }
}

fn unescape_xml(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after_amp = &rest[amp + 1..];
        let Some(semi) = after_amp.find(';') else {
            out.push('&');
            rest = after_amp;
            continue;
        };
        let entity = &after_amp[..semi];
        match entity {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            code if code.starts_with("#x") || code.starts_with("#X") => {
                if let Some(ch) = u32::from_str_radix(&code[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
                {
                    out.push(ch);
                } else {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
            }
            code if code.starts_with('#') => {
                if let Some(ch) = code[1..].parse::<u32>().ok().and_then(char::from_u32) {
                    out.push(ch);
                } else {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
            }
            _ => {
                out.push('&');
                out.push_str(entity);
                out.push(';');
            }
        }
        rest = &after_amp[semi + 1..];
    }
    out.push_str(rest);
    out
}
