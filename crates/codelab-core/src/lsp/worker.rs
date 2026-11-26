use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::execution::{Directory, Process, spawn_piped};
use crate::{AppEvent, CourseLanguage};

use super::{
    Emit, IDLE_TIMEOUT, Language, LanguageDiagnostics, LanguageFeature, LanguageFeatureRequest,
    LanguageFeatureResult, LanguageStatus, REQUEST_TIMEOUT, Source, file_uri,
    protocol::decode_list, status_event,
};

const START_TIMEOUT: Duration = Duration::from_secs(5);
const JAVA_START_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const MAX_FRAME: usize = 8 * 1024 * 1024;
const MAX_HEADER: usize = 4096;
const MAX_REQUESTS: usize = 8;

struct Query {
    request: LanguageFeatureRequest,
    reply: mpsc::SyncSender<LanguageFeatureResult>,
}

struct Focus {
    document: Mutex<Option<Arc<Source>>>,
    stop: AtomicBool,
}

pub(super) struct Worker {
    focus: Arc<Focus>,
    queries: mpsc::SyncSender<Query>,
    thread: Option<JoinHandle<()>>,
    pub last_used: Instant,
}

impl Worker {
    pub fn start(
        executable: PathBuf,
        directory: Directory,
        language: &Language,
        source: Arc<Source>,
        emit: Emit,
    ) -> io::Result<Self> {
        let focus = Arc::new(Focus {
            document: Mutex::new(Some(source)),
            stop: AtomicBool::new(false),
        });
        let control = Arc::clone(&focus);
        let language = language.course;
        let (queries, requests) = mpsc::sync_channel(MAX_REQUESTS);
        let thread = thread::Builder::new()
            .name("codelab-lsp".into())
            .spawn(move || {
                let result = serve(executable, directory, language, &control, requests, &emit);
                let document = control.document.lock().unwrap();
                if !control.stop.swap(true, Ordering::AcqRel)
                    && result.is_err()
                    && let Some(source) = document.as_ref()
                {
                    status_event(&emit, &source.document, language, LanguageStatus::Degraded);
                }
            })?;
        Ok(Self {
            focus,
            queries,
            thread: Some(thread),
            last_used: Instant::now(),
        })
    }

    pub fn focus(&self, source: Option<Arc<Source>>) -> bool {
        let mut document = self.focus.document.lock().unwrap();
        if self.finished() {
            return false;
        }
        *document = source;
        true
    }

    pub fn request(
        &self,
        request: LanguageFeatureRequest,
    ) -> Option<mpsc::Receiver<LanguageFeatureResult>> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.queries.try_send(Query { request, reply }).ok()?;
        Some(receiver)
    }

    pub fn finished(&self) -> bool {
        self.focus.stop.load(Ordering::Acquire)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.focus.stop.store(true, Ordering::Release);
        // The supervisor terminates the process tree before it returns.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    executable: PathBuf,
    directory: Directory,
    language: CourseLanguage,
    focus: &Focus,
    queries: mpsc::Receiver<Query>,
    emit: &Emit,
) -> io::Result<()> {
    let uri = file_uri(&directory.path).ok_or(io::ErrorKind::InvalidInput)?;
    let name = directory
        .path
        .file_name()
        .unwrap_or(directory.path.as_os_str())
        .to_string_lossy()
        .into_owned();
    // JDT LS requires exclusive Eclipse workspace metadata per instance.
    let java_data = (language == CourseLanguage::Java)
        .then(tempfile::tempdir)
        .transpose()?;
    let args = match &java_data {
        Some(data) => vec![
            "-configuration".into(),
            data.path().join("configuration").into_os_string(),
            "-data".into(),
            data.path().join("workspace").into_os_string(),
        ],
        None if matches!(language, CourseLanguage::JsTs | CourseLanguage::Python) => {
            vec!["--stdio".into()]
        }
        None => vec![],
    };
    if focus.stop.load(Ordering::Acquire) {
        return Ok(());
    }
    let (process, (reader, mut writer)) = spawn_piped(executable, args, directory)?;
    let (incoming, messages) = mpsc::sync_channel(4);
    let (output, outgoing) = mpsc::sync_channel::<Value>(MAX_REQUESTS + 8);
    let read_thread = thread::Builder::new()
        .name("codelab-lsp-read".into())
        .spawn(move || {
            let mut reader = BufReader::new(reader);
            while let Ok(message) = read_frame(&mut reader) {
                if incoming.send(message).is_err() {
                    break;
                }
            }
        })?;
    let write_thread = thread::Builder::new()
        .name("codelab-lsp-write".into())
        .spawn(move || {
            for message in outgoing {
                let Ok(body) = serde_json::to_vec(&message) else {
                    break;
                };
                if write!(writer, "Content-Length: {}\r\n\r\n", body.len())
                    .and_then(|_| writer.write_all(&body))
                    .is_err()
                {
                    break;
                }
            }
        });
    let result = match &write_thread {
        Ok(_) => Connection::new(&output, uri, name, language)
            .run(focus, &queries, emit, &messages, &process),
        Err(_) => Err(io::Error::other("language server writer unavailable")),
    };
    drop(output);
    drop(messages);
    drop(process);
    // Normal pipe pumps exit when the process tree closes its handles. Bound
    // the wait even if an already-disowned process has retained a pipe handle.
    let deadline = Instant::now() + POLL_INTERVAL * 2;
    for pump in write_thread.into_iter().chain([read_thread]) {
        while !pump.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        if pump.is_finished() {
            let _ = pump.join();
        }
    }
    result
}

struct Pending {
    feature: LanguageFeature,
    since: Instant,
    reply: mpsc::SyncSender<LanguageFeatureResult>,
}

struct Connection<'a> {
    output: &'a mpsc::SyncSender<Value>,
    uri: url::Url,
    name: String,
    language: CourseLanguage,
    source: Option<Arc<Source>>,
    initialized: bool,
    sync_kind: u64,
    completion: bool,
    hover: bool,
    next_id: u64,
    pending: HashMap<u64, Pending>,
}

impl<'a> Connection<'a> {
    fn new(
        output: &'a mpsc::SyncSender<Value>,
        uri: url::Url,
        name: String,
        language: CourseLanguage,
    ) -> Self {
        Self {
            output,
            uri,
            name,
            language,
            source: None,
            initialized: false,
            sync_kind: 0,
            completion: false,
            hover: false,
            next_id: 1,
            pending: HashMap::new(),
        }
    }

    fn run(
        &mut self,
        focus: &Focus,
        queries: &mpsc::Receiver<Query>,
        emit: &Emit,
        messages: &mpsc::Receiver<Value>,
        process: &Process,
    ) -> io::Result<()> {
        self.send(json!({
            "id": 1, "method": "initialize",
            "params": {
                "processId": std::process::id(), "rootUri": self.uri.as_str(),
                "workspaceFolders": self.folders(),
                "capabilities": {
                    "general": { "positionEncodings": ["utf-16"] },
                    "workspace": { "configuration": true, "workspaceFolders": true },
                    "textDocument": {
                        "completion": { "completionItem": { "snippetSupport": true } },
                        "hover": { "contentFormat": ["markdown", "plaintext"] },
                        "publishDiagnostics": { "versionSupport": true }
                    }
                },
                "initializationOptions": {
                    "workspaceFolders": [self.uri.as_str()]
                }
            }
        }))?;
        let timeout = if self.language == CourseLanguage::Java {
            JAVA_START_TIMEOUT
        } else {
            START_TIMEOUT
        };
        let started = Instant::now();
        let mut last_active = started;
        while !focus.stop.load(Ordering::Acquire) {
            if process.exited() {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let next = {
                let document = focus.document.lock().unwrap();
                if document.is_some() {
                    last_active = Instant::now();
                } else if last_active.elapsed() >= IDLE_TIMEOUT {
                    focus.stop.store(true, Ordering::Release);
                    return Ok(());
                }
                document.clone()
            };
            if self.initialized {
                self.sync(next, emit)?;
                for query in queries.try_iter().take(MAX_REQUESTS) {
                    self.query(query)?;
                }
                self.pending
                    .retain(|_, request| request.since.elapsed() < REQUEST_TIMEOUT);
            } else if started.elapsed() >= timeout {
                return Err(io::ErrorKind::TimedOut.into());
            } else {
                // Completion and hover can be retried once language support is ready.
                for _ in queries.try_iter().take(MAX_REQUESTS) {}
            }
            match messages.recv_timeout(POLL_INTERVAL) {
                Ok(message) => self.message(&message, emit)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
            }
        }
        Ok(())
    }

    fn send(&self, mut message: Value) -> io::Result<()> {
        message["jsonrpc"] = json!("2.0");
        self.output
            .try_send(message)
            .map_err(|_| io::ErrorKind::BrokenPipe.into())
    }

    fn folders(&self) -> Value {
        json!([{ "uri": self.uri.as_str(), "name": self.name }])
    }

    fn message(&mut self, message: &Value, emit: &Emit) -> io::Result<()> {
        if let Some(method) = message["method"].as_str() {
            if let Some(id) = message.get("id") {
                let result = match method {
                    "workspace/configuration" => json!(vec![json!({}); message["params"]["items"].as_array().map_or(0, Vec::len)]),
                    "workspace/workspaceFolders" => self.folders(),
                    "window/workDoneProgress/create" => Value::Null,
                    "workspace/applyEdit" => json!({ "applied": false }),
                    _ => return self.send(json!({ "id": id, "error": { "code": -32601, "message": "Method not supported" } })),
                };
                self.send(json!({ "id": id, "result": result }))?;
            } else if method == "textDocument/publishDiagnostics"
                && let Some(source) = &self.source
            {
                let params = &message["params"];
                if params["uri"]
                    .as_str()
                    .and_then(|uri| url::Url::parse(uri).ok())
                    .as_ref()
                    == Some(&source.uri)
                    && params["version"]
                        .as_i64()
                        .is_none_or(|version| version == i64::from(source.document.version))
                    && params["diagnostics"].is_array()
                {
                    emit(AppEvent::LanguageDiagnostics(LanguageDiagnostics {
                        document: source.document.clone(),
                        diagnostics: decode_list(params["diagnostics"].as_array().unwrap().clone()),
                    }));
                }
            }
        } else if let Some(id) = message["id"].as_u64() {
            if id == 1 && !self.initialized {
                let capabilities = &message["result"]["capabilities"];
                if !capabilities.is_object()
                    || capabilities["positionEncoding"]
                        .as_str()
                        .is_some_and(|encoding| encoding != "utf-16")
                {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                let sync = &capabilities["textDocumentSync"];
                self.sync_kind = sync
                    .as_u64()
                    .or_else(|| sync["change"].as_u64())
                    .unwrap_or(0);
                self.completion = capabilities["completionProvider"].is_object();
                self.hover = capabilities["hoverProvider"]
                    .as_bool()
                    .unwrap_or_else(|| capabilities["hoverProvider"].is_object());
                self.send(json!({ "method": "initialized", "params": {} }))?;
                self.initialized = true;
            } else if let Some(request) = self.pending.remove(&id)
                && let Some(result) =
                    LanguageFeatureResult::parse(request.feature, message["result"].clone())
            {
                let _ = request.reply.try_send(result);
            }
        }
        Ok(())
    }

    fn sync(&mut self, next: Option<Arc<Source>>, emit: &Emit) -> io::Result<()> {
        if self.source.as_ref().map(|s| s.document.version)
            == next.as_ref().map(|s| s.document.version)
        {
            return Ok(());
        }
        let previous = self.source.take();
        let same_uri = previous
            .as_ref()
            .zip(next.as_ref())
            .is_some_and(|(old, new)| old.uri == new.uri);
        if let Some(old) = &previous
            && !same_uri
        {
            self.send(json!({ "method": "textDocument/didClose", "params": {
                "textDocument": { "uri": old.uri.as_str() }
            }}))?;
        }
        if let Some(source) = &next {
            if !same_uri {
                self.send(
                    json!({ "method": "textDocument/didOpen", "params": { "textDocument": {
                        "uri": source.uri.as_str(), "languageId": source.language_id,
                        "version": source.document.version, "text": source.text
                    }}}),
                )?;
            } else if self.sync_kind != 0
                && let Some(old) = previous
            {
                let mut change = json!({ "text": source.text });
                if self.sync_kind == 2 {
                    change["range"] = json!({ "start": { "line": 0, "character": 0 }, "end": end_position(&old.text) });
                }
                self.send(json!({ "method": "textDocument/didChange", "params": {
                    "textDocument": { "uri": source.uri.as_str(), "version": source.document.version },
                    "contentChanges": [change]
                }}))?;
            }
            status_event(emit, &source.document, self.language, LanguageStatus::Ready);
        }
        self.source = next;
        Ok(())
    }

    fn query(&mut self, query: Query) -> io::Result<()> {
        let request = query.request;
        let method = match request.feature {
            LanguageFeature::Completion if self.completion => "textDocument/completion",
            LanguageFeature::Hover if self.hover => "textDocument/hover",
            _ => return Ok(()),
        };
        let Some(source) = self
            .source
            .as_ref()
            .filter(|s| s.document.version == request.version)
        else {
            return Ok(());
        };
        if self.pending.len() >= MAX_REQUESTS {
            return Ok(());
        }
        self.next_id += 1;
        let message = json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": {
            "textDocument": { "uri": source.uri.as_str() },
            "position": { "line": request.line, "character": request.character },
        }});
        match self.output.try_send(message) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => return Ok(()),
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
        }
        self.pending.insert(
            self.next_id,
            Pending {
                feature: request.feature,
                since: Instant::now(),
                reply: query.reply,
            },
        );
        Ok(())
    }
}

fn end_position(text: &str) -> Value {
    let (mut line, mut character, mut cr) = (0, 0, false);
    for ch in text.chars() {
        match ch {
            '\n' if cr => (),
            '\n' | '\r' => {
                line += 1;
                character = 0;
            }
            _ => character += ch.len_utf16(),
        }
        cr = ch == '\r';
    }
    json!({ "line": line, "character": character })
}

fn read_frame(reader: &mut impl BufRead) -> io::Result<Value> {
    use std::io::Read;
    let mut header = Vec::new();
    let mut remaining = MAX_HEADER;
    let mut length = None;
    loop {
        header.clear();
        let read = reader
            .take(remaining as u64)
            .read_until(b'\n', &mut header)?;
        if read == 0 || read == remaining || !header.ends_with(b"\r\n") {
            return Err(io::ErrorKind::InvalidData.into());
        }
        remaining -= read;
        if header == b"\r\n" {
            break;
        }
        if let Some((name, value)) = std::str::from_utf8(&header)
            .ok()
            .and_then(|line| line.split_once(':'))
            && name.eq_ignore_ascii_case("Content-Length")
        {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let size = length
        .filter(|size| *size <= MAX_FRAME)
        .ok_or(io::ErrorKind::InvalidData)?;
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    serde_json::from_slice(&buffer).map_err(io::Error::other)
}
