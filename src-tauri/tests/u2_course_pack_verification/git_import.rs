use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::{isolated_state_with_builtins, repo_builtin_courses_dir, write_sample_pack};
use codelab_studio_lib::course::{
    fetch_git_pack, load_git_pack_dir, load_local_pack_dir, validate_git_ref, validate_git_url,
    GitImportConfig, DEFAULT_MAX_GIT_METADATA_BYTES,
};
use codelab_studio_lib::domain::course::{
    ChapterKind, CourseSourceInput, OpenWorkspaceReq, ParsedCourse,
};
use codelab_studio_lib::domain::error::ErrorCode;

struct SmartHttpGitServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl SmartHttpGitServer {
    fn start(repos: HashMap<String, PathBuf>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback smart-http listener");
        let addr = listener.local_addr().expect("listener local addr");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let repos = Arc::new(repos);

        let handle = thread::spawn(move || {
            for incoming in listener.incoming() {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = incoming else {
                    continue;
                };
                let repos_ref = Arc::clone(&repos);
                thread::spawn(move || {
                    let _ = handle_git_http_connection(stream, &repos_ref);
                });
            }
        });

        Self {
            addr,
            stop,
            handle: Some(handle),
        }
    }

    fn url_for(&self, repo_name: &str) -> String {
        format!("http://{}/{repo_name}", self.addr)
    }
}

impl Drop for SmartHttpGitServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct StalledTcpServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl StalledTcpServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled listener");
        let addr = listener.local_addr().expect("stalled addr");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);

        let handle = thread::spawn(move || {
            let mut held_streams = Vec::new();
            for incoming in listener.incoming() {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = incoming {
                    held_streams.push(stream);
                }
            }
            drop(held_streams);
        });

        Self {
            addr,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}/stalled.git", self.addr)
    }
}

impl Drop for StalledTcpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|window| window == b"\r\n")
}

fn read_chunked_body(stream: &mut TcpStream, initial: Vec<u8>) -> std::io::Result<Vec<u8>> {
    let mut buf = initial;
    let mut cursor = 0usize;
    let mut out = Vec::new();
    let mut tmp = [0u8; 4096];

    loop {
        let line_end = loop {
            if let Some(pos) = find_crlf(&buf[cursor..]) {
                break cursor + pos;
            }
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                return Ok(out);
            }
            buf.extend_from_slice(&tmp[..n]);
        };

        let size_token = std::str::from_utf8(&buf[cursor..line_end])
            .unwrap_or("0")
            .split(';')
            .next()
            .unwrap_or("0")
            .trim();
        let chunk_size = usize::from_str_radix(size_token, 16).unwrap_or(0);
        cursor = line_end + 2;

        if chunk_size == 0 {
            break;
        }

        let needed = cursor + chunk_size + 2;
        while buf.len() < needed {
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let end = (cursor + chunk_size).min(buf.len());
        out.extend_from_slice(&buf[cursor..end]);
        cursor = (cursor + chunk_size + 2).min(buf.len());
    }

    Ok(out)
}

fn handle_git_http_connection(
    mut stream: TcpStream,
    repos: &HashMap<String, PathBuf>,
) -> std::io::Result<()> {
    let mut raw = Vec::new();
    let mut tmp = [0u8; 4096];
    let header_end = loop {
        if let Some(pos) = find_header_end(&raw) {
            break pos;
        }
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        raw.extend_from_slice(&tmp[..n]);
    };

    let header_str = String::from_utf8_lossy(&raw[..header_end]);
    let mut lines = header_str.split("\r\n");
    let Some(request_line) = lines.next() else {
        return Ok(());
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    let mut expect_continue = false;
    for line in lines {
        if let Some((name, val)) = line.split_once(':') {
            let lower = name.trim().to_ascii_lowercase();
            let val_trimmed = val.trim();
            if lower == "content-length" {
                content_length = val_trimmed.parse::<usize>().ok();
            } else if lower == "transfer-encoding"
                && val_trimmed.to_ascii_lowercase().contains("chunked")
            {
                is_chunked = true;
            } else if lower == "expect" && val_trimmed.eq_ignore_ascii_case("100-continue") {
                expect_continue = true;
            }
        }
    }

    let (path_only, query) = match target.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (target, None),
    };
    let trimmed_path = path_only.trim_start_matches('/');
    let Some((repo_name, sub_path)) = trimmed_path.split_once('/') else {
        stream.write_all(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    };
    let Some(repo_dir) = repos.get(repo_name) else {
        stream.write_all(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    };

    if method == "GET" && sub_path == "info/refs" && query == Some("service=git-upload-pack") {
        let out = Command::new("git")
            .env_remove("GIT_PROTOCOL")
            .args(["upload-pack", "--stateless-rpc", "--advertise-refs"])
            .arg(repo_dir)
            .output()?;
        let mut body = b"001e# service=git-upload-pack\n0000".to_vec();
        body.extend_from_slice(&out.stdout);
        let resp_head = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/x-git-upload-pack-advertisement\r\n\
             Cache-Control: no-cache\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(resp_head.as_bytes())?;
        stream.write_all(&body)?;
        stream.flush()?;
        return Ok(());
    }

    if method == "POST" && sub_path == "git-upload-pack" {
        if expect_continue {
            stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
            stream.flush()?;
        }

        let leftover = raw[header_end + 4..].to_vec();
        let req_body = if is_chunked {
            read_chunked_body(&mut stream, leftover)?
        } else {
            let expected = content_length.unwrap_or(0);
            let mut body = leftover;
            while body.len() < expected {
                let remaining = expected - body.len();
                let mut chunk = vec![0u8; remaining.min(4096)];
                let n = stream.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..n]);
            }
            body.truncate(expected);
            body
        };

        let mut child = Command::new("git")
            .env_remove("GIT_PROTOCOL")
            .args(["upload-pack", "--stateless-rpc"])
            .arg(repo_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&req_body)?;
        }
        let out = child.wait_with_output()?;
        let resp_head = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/x-git-upload-pack-result\r\n\
             Cache-Control: no-cache\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n",
            out.stdout.len()
        );
        stream.write_all(resp_head.as_bytes())?;
        stream.write_all(&out.stdout)?;
        stream.flush()?;
        return Ok(());
    }

    stream
        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
    Ok(())
}

fn run_git_ok(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("spawn git command");
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_bare_course_repo(root: &Path, name: &str, title: &str, extra_bytes: usize) -> PathBuf {
    let work = root.join(format!("work-{name}"));
    let bare = root.join(format!("{name}.git"));
    fs::create_dir_all(&work).expect("create work repo dir");
    write_sample_pack(&work, title, &format!("starter from {name}\n"));
    if extra_bytes > 0 {
        fs::write(
            work.join("starter").join("payload.bin"),
            vec![b'A'; extra_bytes],
        )
        .expect("write extra payload");
    }

    finish_bare_course_repo(&work, &bare);
    bare
}

fn create_bare_course_repo_from_pack(root: &Path, name: &str, pack_dir: &Path) -> PathBuf {
    let work = root.join(format!("work-{name}"));
    let bare = root.join(format!("{name}.git"));
    fs::create_dir_all(&work).expect("create work repo dir");
    copy_course_pack(pack_dir, &work);
    finish_bare_course_repo(&work, &bare);
    bare
}

fn copy_course_pack(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("read source course pack") {
        let entry = entry.expect("read course pack entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("read course pack entry type");
        assert!(
            !file_type.is_symlink(),
            "course fixture must not contain symlinks"
        );
        if file_type.is_dir() {
            fs::create_dir(&destination_path).expect("create copied pack directory");
            copy_course_pack(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy course pack file");
        }
    }
}

fn finish_bare_course_repo(work: &Path, bare: &Path) {
    run_git_ok(work, &["init"]);
    run_git_ok(work, &["checkout", "-b", "main"]);
    run_git_ok(work, &["config", "user.name", "CodeLab Test"]);
    run_git_ok(work, &["config", "user.email", "test@codelab.local"]);
    run_git_ok(work, &["config", "commit.gpgsign", "false"]);

    run_git_ok(work, &["add", "-A"]);
    run_git_ok(
        work,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "initial commit",
        ],
    );

    let clone_out = Command::new("git")
        .args(["clone", "--bare"])
        .arg(work)
        .arg(bare)
        .output()
        .expect("create bare clone");
    assert!(
        clone_out.status.success(),
        "bare clone failed: {}",
        String::from_utf8_lossy(&clone_out.stderr)
    );
}

fn start_single_course_server(
    repo_name: &str,
    title: &str,
    extra_bytes: usize,
) -> (tempfile::TempDir, SmartHttpGitServer, String) {
    let repos_root = tempfile::tempdir().expect("repos root");
    let bare_repo = create_bare_course_repo(repos_root.path(), repo_name, title, extra_bytes);
    let repo_file_name = format!("{repo_name}.git");
    let repos = HashMap::from([(repo_file_name.clone(), bare_repo)]);
    let server = SmartHttpGitServer::start(repos);
    let url = server.url_for(&repo_file_name);
    (repos_root, server, url)
}

#[test]
fn git_url_and_ref_validation_rejects_unsafe_syntax_and_credentials() {
    assert!(validate_git_url("https://github.com/org/pack.git").is_ok());
    assert!(validate_git_url("ssh://git@github.com/org/pack.git").is_ok());
    assert!(validate_git_url("git@github.com:org/pack.git").is_ok());
    for url in [
        "https://user:token@github.com/org/pack.git",
        "https://token@github.com/org/pack.git",
        "ssh://git:token@github.com/org/pack.git",
        "https://github.com/org/pack.git?access_token=secret",
        "https://github.com/org/pack.git#access_token=secret",
    ] {
        assert_eq!(
            validate_git_url(url).unwrap_err().code,
            ErrorCode::SchemaValidationFailed,
            "URL should be rejected: {url}"
        );
    }
    assert_eq!(
        validate_git_url("file:///etc/passwd").unwrap_err().code,
        ErrorCode::SchemaValidationFailed
    );
    assert_eq!(
        validate_git_url("ext::sh -c id").unwrap_err().code,
        ErrorCode::SchemaValidationFailed
    );
    assert_eq!(
        validate_git_url("git://github.com/org/pack.git")
            .unwrap_err()
            .code,
        ErrorCode::SchemaValidationFailed
    );
    assert_eq!(
        validate_git_ref("--upload-pack=evil").unwrap_err().code,
        ErrorCode::SchemaValidationFailed
    );
    assert!(validate_git_ref("main").is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn git_import_cache_uses_shallow_clone_and_lru_eviction() {
    let repos_root = tempfile::tempdir().expect("repos root");
    let bare_a = create_bare_course_repo(repos_root.path(), "repo-a", "Git Pack A", 0);
    let bare_b = create_bare_course_repo(repos_root.path(), "repo-b", "Git Pack B", 0);
    let bare_c = create_bare_course_repo(repos_root.path(), "repo-c", "Git Pack C", 0);

    let mut repos = HashMap::new();
    repos.insert("repo-a.git".to_owned(), bare_a);
    repos.insert("repo-b.git".to_owned(), bare_b);
    repos.insert("repo-c.git".to_owned(), bare_c);

    let server = SmartHttpGitServer::start(repos);
    let url_a = server.url_for("repo-a.git");
    let url_b = server.url_for("repo-b.git");
    let url_c = server.url_for("repo-c.git");
    let cache_dir = tempfile::tempdir().expect("git cache dir");
    let lru_cfg = GitImportConfig {
        clone_timeout: Duration::from_secs(15),
        max_pack_bytes: 1024 * 1024,
        max_git_metadata_bytes: DEFAULT_MAX_GIT_METADATA_BYTES,
        max_cache_packs: 2,
        max_cache_total_bytes: 10 * 1024 * 1024,
    };

    let res_a1 = fetch_git_pack(cache_dir.path(), &url_a, Some("main"), false, &lru_cfg)
        .await
        .expect("fetch repo-a");
    assert!(!res_a1.cache_hit);
    assert!(res_a1.head_commit.is_some());
    assert!(res_a1.cache_dir.is_dir());
    assert!(
        !res_a1.cache_dir.join(".git").exists(),
        ".git metadata must be stripped after shallow clone"
    );

    let res_b = fetch_git_pack(cache_dir.path(), &url_b, Some("main"), false, &lru_cfg)
        .await
        .expect("fetch repo-b");
    assert!(!res_b.cache_hit);
    assert!(res_b.cache_dir.is_dir());

    let res_a_hit = fetch_git_pack(cache_dir.path(), &url_a, Some("main"), false, &lru_cfg)
        .await
        .expect("cache hit for repo-a");
    assert!(res_a_hit.cache_hit);
    assert_eq!(res_a_hit.cache_dir, res_a1.cache_dir);

    let res_c = fetch_git_pack(cache_dir.path(), &url_c, Some("main"), false, &lru_cfg)
        .await
        .expect("fetch repo-c triggers LRU eviction of repo-b");
    assert!(!res_c.cache_hit);
    assert!(res_a1.cache_dir.is_dir(), "MRU repo-a must stay cached");
    assert!(res_c.cache_dir.is_dir(), "new repo-c must be cached");
    assert!(
        !res_b.cache_dir.exists(),
        "LRU repo-b must be evicted when exceeding max_cache_packs"
    );

    let res_a_refresh = fetch_git_pack(cache_dir.path(), &url_a, Some("main"), true, &lru_cfg)
        .await
        .expect("refresh bypasses cache");
    assert!(!res_a_refresh.cache_hit);
    assert!(res_a_refresh.cache_dir.is_dir());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn git_import_rejects_oversized_repository_and_cleans_staging() {
    let (_repos_root, _server, url) = start_single_course_server("repo-big", "Big Pack", 16 * 1024);
    let small_cap_cfg = GitImportConfig {
        clone_timeout: Duration::from_secs(15),
        max_pack_bytes: 4096,
        max_git_metadata_bytes: DEFAULT_MAX_GIT_METADATA_BYTES,
        max_cache_packs: 4,
        max_cache_total_bytes: 10 * 1024 * 1024,
    };
    let size_cache = tempfile::tempdir().expect("size cap cache");
    let size_err = fetch_git_pack(size_cache.path(), &url, Some("main"), false, &small_cap_cfg)
        .await
        .expect_err("oversized git repo must be rejected");
    assert_eq!(size_err.code, ErrorCode::FileTooLargeOrBinary);
    let leftover_staging: Vec<_> = fs::read_dir(size_cache.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".staging-"))
        .collect();
    assert!(leftover_staging.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn git_import_enforces_temporary_git_metadata_cap_separately() {
    let (_repos_root, _server, url) = start_single_course_server("repo-metadata", "Metadata", 0);
    let config = GitImportConfig {
        max_pack_bytes: 1024 * 1024,
        max_git_metadata_bytes: 1,
        ..Default::default()
    };
    let cache_dir = tempfile::tempdir().expect("metadata cap cache");
    let error = fetch_git_pack(cache_dir.path(), &url, Some("main"), false, &config)
        .await
        .expect_err("Git metadata should exceed its own cap");

    assert_eq!(error.code, ErrorCode::FileTooLargeOrBinary);
    assert!(error
        .detail
        .as_deref()
        .unwrap_or_default()
        .contains("Git metadata"));
    assert!(fs::read_dir(cache_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .all(|entry| !entry.file_name().to_string_lossy().starts_with(".staging-")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn git_import_stops_a_stalled_clone_after_timeout() {
    let stalled = StalledTcpServer::start();
    let timeout_cache = tempfile::tempdir().expect("timeout cache");
    let timeout_cfg = GitImportConfig {
        clone_timeout: Duration::from_millis(250),
        ..Default::default()
    };
    let started = Instant::now();
    let timeout_err = fetch_git_pack(
        timeout_cache.path(),
        &stalled.url(),
        None,
        false,
        &timeout_cfg,
    )
    .await
    .expect_err("stalled server must hit clone timeout");
    assert_eq!(
        timeout_err.code,
        ErrorCode::SchemaValidationFailed,
        "{timeout_err:?}"
    );
    assert_eq!(timeout_err.message_key, "error.gitImportFailed");
    assert!(timeout_err.retryable);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_state_trusts_git_course_and_hydrates_starter() {
    let (_repos_root, _server, url) = start_single_course_server("repo-a", "Git Pack A", 0);
    let git_config = GitImportConfig::default();
    let (_home, state) = isolated_state_with_builtins();
    let ws = tempfile::tempdir().expect("git ws");
    let git_source = CourseSourceInput::GitRepo {
        url: url.clone(),
        git_ref: Some("main".to_owned()),
        refresh: false,
    };
    let inspect = state
        .inspect_course_source_with_git_config(git_source.clone(), &git_config)
        .await
        .expect("inspect git repo through AppState");
    assert!(inspect.requires_trust);
    assert!(!inspect.trusted);
    assert_eq!(inspect.course.title, "Git Pack A");

    let opened = state
        .trust_and_open_course_with_git_config(
            OpenWorkspaceReq {
                source: git_source,
                workspace_root: ws.path().to_string_lossy().into_owned(),
                create_if_missing: false,
                accept_trust: true,
                expected_fingerprint: Some(inspect.pack_fingerprint),
            },
            &git_config,
        )
        .await
        .expect("trust and open git course pack");
    assert_eq!(
        fs::read_to_string(ws.path().join("hello.txt")).expect("read hydrated git starter"),
        "starter from repo-a\n"
    );
    state
        .reload_active_manual(opened.epoch)
        .await
        .expect("reload unchanged Git pack");
    let work = _repos_root.path().join("work-repo-a");
    write_sample_pack(&work, "Updated Git Pack A", "starter from repo-a\n");
    run_git_ok(&work, &["add", "-A"]);
    run_git_ok(&work, &["commit", "-m", "update course fixture"]);
    let bare_repo = _repos_root.path().join("repo-a.git");
    run_git_ok(&work, &["push", bare_repo.to_str().unwrap(), "main"]);
    let refreshed = state
        .inspect_course_source(CourseSourceInput::GitRepo {
            url,
            git_ref: Some("main".to_owned()),
            refresh: true,
        })
        .await
        .expect("inspect updated pack");
    assert_eq!(refreshed.course.title, "Updated Git Pack A");
    assert!(!refreshed.trusted);
    let error = state
        .reload_active_manual(opened.epoch)
        .await
        .expect_err("changed Git pack needs trust");
    assert_eq!(error.code, ErrorCode::ExternalPackUntrusted);
    state
        .close_workspace(opened.epoch)
        .await
        .expect("close git workspace");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "manual release performance checkpoint"]
async fn benchmark_go_course_local_and_cached_git_pack_loading() {
    let runs = std::env::var("CODELAB_U2_BENCH_RUNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);
    assert!((1..=10_000).contains(&runs));

    let builtin_pack = repo_builtin_courses_dir().join("go-goframe-practice");
    let repos_root = tempfile::tempdir().expect("benchmark repos root");
    let bare_repo =
        create_bare_course_repo_from_pack(repos_root.path(), "go-goframe-practice", &builtin_pack);
    let repo_name = "go-goframe-practice.git";
    let server = SmartHttpGitServer::start(HashMap::from([(repo_name.to_owned(), bare_repo)]));
    let url = server.url_for(repo_name);
    let cache_dir = tempfile::tempdir().expect("benchmark cache dir");
    let config = GitImportConfig::default();
    let primed = fetch_git_pack(cache_dir.path(), &url, Some("main"), false, &config)
        .await
        .expect("prime Git pack cache");
    assert!(!primed.cache_hit);

    let mut local_samples_ms = Vec::with_capacity(runs);
    let mut cached_git_samples_ms = Vec::with_capacity(runs);
    let mut cache_hits = 0;
    for _ in 0..runs {
        let started = Instant::now();
        let local = load_local_pack_dir(&builtin_pack).expect("load local Go course pack");
        assert_go_course_shape(&local.course);
        assert_eq!(
            local.course.title,
            "Go 与 GoFrame 开发实践手册：从第一性原理出发"
        );
        local_samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);

        let started = Instant::now();
        let cached = fetch_git_pack(cache_dir.path(), &url, Some("main"), false, &config)
            .await
            .expect("read cached Git pack");
        assert!(cached.cache_hit);
        cache_hits += usize::from(cached.cache_hit);
        let git_pack = load_git_pack_dir(
            &cached.cache_dir,
            &url,
            Some("main".to_owned()),
            cached.source_key,
        )
        .expect("load cached Git course pack");
        assert_go_course_shape(&git_pack.course);
        assert_eq!(git_pack.course.title, local.course.title);
        cached_git_samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    println!(
        "Go/GoFrame pack benchmark: runs={runs} chapters=15 exercises=10 cacheHits={cache_hits}/{runs} cachedBytes={} local_ms={} cached_git_ms={}",
        primed.size_bytes,
        summarize_samples(&local_samples_ms),
        summarize_samples(&cached_git_samples_ms),
    );
}

fn assert_go_course_shape(course: &ParsedCourse) {
    assert_eq!(course.chapters.len(), 15);
    let exercises: Vec<_> = course
        .chapters
        .iter()
        .filter(|chapter| chapter.kind == ChapterKind::Exercise)
        .collect();
    let readings: Vec<_> = course
        .chapters
        .iter()
        .filter(|chapter| chapter.kind == ChapterKind::Reading)
        .collect();
    assert_eq!(exercises.len(), 10);
    assert_eq!(readings.len(), 5);
    let exercise_ids: Vec<_> = exercises
        .iter()
        .map(|chapter| chapter.chapter_id.0.as_str())
        .collect();
    let expected_ids: Vec<_> = (0..10).map(|index| format!("ex-{index}")).collect();
    assert_eq!(
        exercise_ids,
        expected_ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

fn summarize_samples(samples: &[f64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    };
    format!(
        "median={median:.3}ms range={:.3}..{:.3}ms",
        sorted[0],
        sorted[sorted.len() - 1]
    )
}
