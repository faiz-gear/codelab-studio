use std::collections::VecDeque;

use base64::{Engine, engine::general_purpose::STANDARD};

use super::model::TerminalOutput;

const SCROLLBACK_LINES: usize = 5_000;
const BUFFER_BYTES: usize = 4 * 1024 * 1024;
const CHUNK_BYTES: usize = 128 * 1024;
const ERROR_LINES: usize = 200;
const MAX_LINE_BYTES: usize = 8 * 1024;

#[derive(Default)]
pub(super) struct Capture {
    bytes: VecDeque<u8>,
    end: u64,
    newlines: VecDeque<u64>,
    line: Vec<u8>,
    pub port_conflict: bool,
    pub observed_port: Option<u16>,
}

impl Capture {
    pub fn append(&mut self, bytes: &[u8], errors: &mut VecDeque<String>) {
        for &byte in bytes {
            self.bytes.push_back(byte);
            self.end += 1;
            if byte == b'\n' {
                self.newlines.push_back(self.end);
            }
            match byte {
                b'\n' | b'\r' => self.finish_line(errors),
                _ if self.line.len() < MAX_LINE_BYTES => self.line.push(byte),
                _ => {}
            }
        }
        let mut start = self.end.saturating_sub(BUFFER_BYTES as u64);
        while self.newlines.len() > SCROLLBACK_LINES {
            start = start.max(self.newlines.pop_front().unwrap());
        }
        while self
            .newlines
            .front()
            .is_some_and(|newline| *newline <= start)
        {
            self.newlines.pop_front();
        }
        let to_drop = start.saturating_sub(self.end - self.bytes.len() as u64);
        self.bytes.drain(..to_drop as usize);
    }

    pub fn finish_line(&mut self, errors: &mut VecDeque<String>) {
        if self.line.is_empty() {
            return;
        }
        let clean = strip_ansi_escapes::strip(std::mem::take(&mut self.line));
        let text = String::from_utf8_lossy(&clean);
        let lower = text.to_ascii_lowercase();
        let conflict = [
            "address already in use",
            "eaddrinuse",
            "only one usage of each socket address",
            "port is already allocated",
        ]
        .iter()
        .any(|message| lower.contains(message));
        self.port_conflict |= conflict;
        if conflict
            || ["listening", "running on", "started server"]
                .iter()
                .any(|marker| lower.contains(marker))
        {
            self.observed_port = reported_port(&lower).or(self.observed_port);
        }
        if conflict
            || [
                "error",
                "failed",
                "exception",
                "panic",
                "fatal",
                "traceback",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            errors.push_back(text.into_owned());
            if errors.len() > ERROR_LINES {
                errors.pop_front();
            }
        }
    }

    pub fn output(&self, cursor: u64) -> TerminalOutput {
        let start = self.end - self.bytes.len() as u64;
        let reset = cursor < start || cursor > self.end;
        let offset = if reset { start } else { cursor };
        let length = ((self.end - offset) as usize).min(CHUNK_BYTES);
        let data: Vec<_> = self
            .bytes
            .range((offset - start) as usize..(offset - start) as usize + length)
            .copied()
            .collect();
        let cursor = offset + length as u64;
        TerminalOutput {
            cursor,
            reset,
            data: STANDARD.encode(data),
            more: cursor < self.end,
        }
    }
}

fn reported_port(line: &str) -> Option<u16> {
    line.rsplit_once("port ")
        .map(|(_, value)| value)
        .into_iter()
        .chain(
            line.rmatch_indices(':')
                .map(|(index, _)| &line[index + 1..]),
        )
        .filter_map(|value| {
            let digits = value
                .as_bytes()
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            value[..digits].parse::<u16>().ok().filter(|port| *port > 0)
        })
        .next()
}
