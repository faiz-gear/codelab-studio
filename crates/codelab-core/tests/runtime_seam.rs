use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use codelab_core::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, CourseLanguage, EventSubscription,
    HostProbeResult, HostTool, HostToolchainProbe, LanguagePreflight, PreflightReport,
    ProbeProblem, ProbeStatus, RuntimeAdapters, RuntimeSource,
};
use tempfile::TempDir;

struct MixedToolchains {
    concurrent: Barrier,
}

impl HostToolchainProbe for MixedToolchains {
    fn probe(&self, tool: HostTool, timeout: Duration) -> HostProbeResult {
        assert_eq!(timeout, Duration::from_secs(5));
        if matches!(tool, HostTool::Node | HostTool::Go) {
            self.concurrent.wait();
        }
        let output = match tool {
            HostTool::Node => "v20.0.0",
            HostTool::Go => "go version go1.21.13 linux/amd64",
            HostTool::Javac => "javac 21.0.4",
            HostTool::Python3 => "Python 3.10.15",
            HostTool::UvPython => "Python 3.11.9",
            HostTool::Git => "git version 2.50.0",
            HostTool::Curl => return HostProbeResult::TimedOut,
            HostTool::GoProxy => "https://user:secret@proxy.example.invalid,direct",
            HostTool::Gopls => "",
            _ => return HostProbeResult::Missing,
        };
        HostProbeResult::Found {
            output: output.into(),
            executable: Some(PathBuf::from("test-host").join(tool.command())),
        }
    }
}

fn report(app: &App, command: AppCommand) -> PreflightReport {
    match app.execute(command).expect("preflight command") {
        AppResponse::Preflight(report) => report,
        other => panic!("expected Preflight, got {other:?}"),
    }
}

fn wait_for_report(
    events: &EventSubscription,
    matches: impl Fn(&PreflightReport) -> bool,
) -> PreflightReport {
    loop {
        if let AppEvent::PreflightUpdated(report) = events
            .recv_timeout(Duration::from_secs(7))
            .expect("preflight event")
            && matches(&report)
        {
            return report;
        }
    }
}

fn language(report: &PreflightReport, lang: CourseLanguage) -> &LanguagePreflight {
    report
        .languages
        .iter()
        .find(|row| row.language == lang)
        .expect("language row")
}

#[test]
fn preflight_probes_concurrently_and_applies_version_floors_without_requiring_lsp() {
    let dir = TempDir::new().unwrap();
    let app = App::open_with_runtime_adapters(
        AppConfig {
            data_dir: dir.path().to_path_buf(),
        },
        RuntimeAdapters {
            host: Arc::new(MixedToolchains {
                concurrent: Barrier::new(2),
            }),
            ..RuntimeAdapters::default()
        },
    )
    .unwrap();
    let events = app.subscribe();

    assert!(report(&app, AppCommand::RefreshPreflight).probing);
    let checked = wait_for_report(&events, |report| !report.probing && report.checked);
    let javascript = language(&checked, CourseLanguage::JsTs);
    assert_eq!(javascript.status, ProbeStatus::Ok);
    assert_eq!(
        javascript.selected,
        Some(RuntimeSource::System {
            tool: HostTool::Node
        })
    );
    assert_eq!(
        javascript.lsp.as_ref().unwrap().status,
        ProbeStatus::Missing
    );
    assert!(!javascript.can_install);

    let go = language(&checked, CourseLanguage::Go);
    assert_eq!(go.status, ProbeStatus::Warn);
    assert_eq!(go.system[0].problem, Some(ProbeProblem::VersionTooOld));
    assert!(go.can_install);
    assert_eq!(go.lsp.as_ref().unwrap().status, ProbeStatus::Ok);

    assert_eq!(
        language(&checked, CourseLanguage::Java).status,
        ProbeStatus::Ok
    );
    let python = language(&checked, CourseLanguage::Python);
    assert_eq!(python.status, ProbeStatus::Ok);
    assert_eq!(
        python
            .system
            .iter()
            .find(|tool| tool.status == ProbeStatus::Ok)
            .unwrap()
            .tool,
        HostTool::UvPython
    );

    assert_eq!(checked.tools[0].status, ProbeStatus::Ok);
    assert_eq!(checked.tools[1].problem, Some(ProbeProblem::TimedOut));
    let proxy = checked.go_proxy.unwrap();
    assert_eq!(proxy.status, ProbeStatus::Ok);
    assert_eq!(
        proxy.value.as_deref(),
        Some("https://proxy.example.invalid,direct")
    );
}

struct DownloadFixture {
    artifact: codelab_core::RuntimeArtifact,
}

impl codelab_core::RuntimeDownloadAdapter for DownloadFixture {
    fn download(
        &self,
        artifact: &codelab_core::RuntimeArtifact,
        destination: &mut dyn std::io::Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), codelab_core::DomainError> {
        assert!(artifact.url.contains("/oven-sh/bun/"));
        codelab_core::HttpRuntimeDownloader.download(&self.artifact, destination, progress)
    }
}

#[test]
fn preferred_managed_runtime_downloads_once_and_survives_a_course_switch() {
    use std::io::{BufRead, BufReader, Cursor, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    use codelab_core::{InstallProgress, OpenCourseRequest, RuntimeArtifact, RuntimePreference};
    use sha2::{Digest, Sha256};

    let executable = if cfg!(windows) { "bun.exe" } else { "bun" };
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        format!("bun-bundle/{executable}"),
        zip::write::SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"portable bun fixture").unwrap();
    let archive = zip.finish().unwrap().into_inner();
    let sha256 = format!("{:x}", Sha256::digest(&archive));

    let http = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/bun.zip", http.local_addr().unwrap());
    let (resume, paused) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = http.accept().unwrap();
        let mut reader = BufReader::new(&socket);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                break;
            }
        }
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            archive.len()
        )
        .unwrap();
        socket.write_all(&archive[..8]).unwrap();
        paused.recv_timeout(Duration::from_secs(7)).unwrap();
        socket.write_all(&archive[8..]).unwrap();
    });

    let dir = TempDir::new().unwrap();
    let config = AppConfig {
        data_dir: dir.path().join("app"),
    };
    let adapters = || RuntimeAdapters {
        host: Arc::new(MixedToolchains {
            concurrent: Barrier::new(2),
        }),
        downloads: Arc::new(DownloadFixture {
            artifact: RuntimeArtifact {
                url: url.clone(),
                sha256: sha256.clone(),
            },
        }),
    };
    let app = App::open_with_runtime_adapters(config.clone(), adapters()).unwrap();
    let events = app.subscribe();
    report(&app, AppCommand::RefreshPreflight);
    wait_for_report(&events, |report| report.checked);

    let install = AppCommand::InstallManagedRuntime {
        language: CourseLanguage::JsTs,
    };
    let system = report(&app, install.clone());
    assert!(language(&system, CourseLanguage::JsTs).install.is_none());

    let preference = |prefer_managed| {
        AppCommand::SetRuntimePreference(RuntimePreference {
            language: CourseLanguage::JsTs,
            prefer_managed,
        })
    };
    let preferred = report(&app, preference(true));
    assert!(language(&preferred, CourseLanguage::JsTs).can_install);
    assert_eq!(
        language(&preferred, CourseLanguage::JsTs).selected,
        Some(RuntimeSource::System {
            tool: HostTool::Node
        })
    );

    report(&app, install.clone());
    let downloading = wait_for_report(&events, |report| {
        matches!(
            language(report, CourseLanguage::JsTs).install,
            Some(InstallProgress::Downloading { received_bytes, .. }) if received_bytes > 0
        )
    });
    let unchanged = report(&app, install);
    assert_eq!(unchanged.revision, downloading.revision);
    assert!(language(&unchanged, CourseLanguage::JsTs).managed.is_none());

    for name in ["first", "second"] {
        let markdown = dir.path().join(format!("{name}.md"));
        std::fs::write(&markdown, format!("# {name}\n\n## Start\nRead me.")).unwrap();
        assert!(matches!(
            app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
                path: markdown.display().to_string(),
            })),
            Ok(AppResponse::LoadedCourse(_))
        ));
    }
    resume.send(()).unwrap();

    let installed = wait_for_report(&events, |report| {
        matches!(
            language(report, CourseLanguage::JsTs).install,
            Some(InstallProgress::Installed)
        )
    });
    let javascript = language(&installed, CourseLanguage::JsTs);
    assert_eq!(javascript.selected, Some(RuntimeSource::Managed));
    let managed = javascript.managed.as_ref().unwrap();
    assert!(
        managed
            .executable
            .starts_with(config.data_dir.join("runtimes"))
    );
    assert_eq!(managed.test_executor, managed.executable);
    assert_eq!(
        std::fs::read(&managed.executable).unwrap(),
        b"portable bun fixture"
    );

    let restored = report(&app, preference(false));
    assert_eq!(
        language(&restored, CourseLanguage::JsTs).selected,
        Some(RuntimeSource::System {
            tool: HostTool::Node
        })
    );
    report(&app, preference(true));
    drop(app);
    let app = App::open_with_runtime_adapters(config, adapters()).unwrap();
    let saved = report(&app, AppCommand::GetPreflight);
    let saved = language(&saved, CourseLanguage::JsTs);
    assert!(saved.prefer_managed);
    assert_eq!(saved.selected, Some(RuntimeSource::Managed));
    assert_eq!(saved.managed.as_ref(), Some(managed));
    server.join().unwrap();
}

struct SlowProbe;

impl HostToolchainProbe for SlowProbe {
    fn probe(&self, tool: HostTool, _: Duration) -> HostProbeResult {
        let output = match tool {
            HostTool::Bun => "1.3.13",
            HostTool::Node => "v19.9.0",
            HostTool::Go => "go version go1.22.0 linux/amd64",
            HostTool::Javac => "javac 17.0.2",
            HostTool::Python3 => "Python 3.11.0",
            HostTool::GoProxy => "off",
            HostTool::Curl => {
                std::thread::sleep(Duration::from_secs(7));
                "curl 8.0.1"
            }
            _ => return HostProbeResult::Missing,
        };
        HostProbeResult::Found {
            output: output.into(),
            executable: None,
        }
    }
}

#[test]
fn a_stalled_probe_times_out_while_the_other_toolchains_report_their_results() {
    let dir = TempDir::new().unwrap();
    let app = App::open_with_runtime_adapters(
        AppConfig {
            data_dir: dir.path().to_path_buf(),
        },
        RuntimeAdapters {
            host: Arc::new(SlowProbe),
            ..Default::default()
        },
    )
    .unwrap();
    let events = app.subscribe();

    report(&app, AppCommand::RefreshPreflight);
    let checked = wait_for_report(&events, |report| report.checked);
    assert_eq!(checked.tools[1].problem, Some(ProbeProblem::TimedOut));
    assert_eq!(
        checked.go_proxy.as_ref().unwrap().problem,
        Some(ProbeProblem::ProxyDisabled)
    );
    let javascript = language(&checked, CourseLanguage::JsTs);
    assert_eq!(javascript.status, ProbeStatus::Ok);
    assert_eq!(
        javascript.system[1].problem,
        Some(ProbeProblem::VersionTooOld)
    );
    assert_eq!(
        language(&checked, CourseLanguage::Go).status,
        ProbeStatus::Ok
    );
    assert_eq!(
        language(&checked, CourseLanguage::Java).status,
        ProbeStatus::Warn
    );
    assert_eq!(
        language(&checked, CourseLanguage::Python).status,
        ProbeStatus::Ok
    );
}

struct MissingToolchains;

impl HostToolchainProbe for MissingToolchains {
    fn probe(&self, _: HostTool, _: Duration) -> HostProbeResult {
        HostProbeResult::Missing
    }
}

struct LoopbackInstaller {
    origin: String,
    checksums: HashMap<&'static str, String>,
}

impl codelab_core::RuntimeDownloadAdapter for LoopbackInstaller {
    fn download(
        &self,
        artifact: &codelab_core::RuntimeArtifact,
        destination: &mut dyn std::io::Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), codelab_core::DomainError> {
        let name = artifact
            .url
            .rsplit('/')
            .next()
            .unwrap()
            .to_ascii_lowercase();
        let key = self
            .checksums
            .keys()
            .find(|key| name.starts_with(**key))
            .unwrap();
        let fixture = codelab_core::RuntimeArtifact {
            url: format!("{}/{key}", self.origin),
            sha256: self.checksums[key].clone(),
        };
        codelab_core::HttpRuntimeDownloader.download(&fixture, destination, progress)
    }
}

fn fixture_archive(entries: &[(&str, &str)], zip_format: bool) -> Vec<u8> {
    use std::io::{Cursor, Write};

    if zip_format {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, contents) in entries {
            zip.start_file(
                *path,
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        return zip.finish().unwrap().into_inner();
    }
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(gzip);
    for (path, contents) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(contents.len() as u64);
        tar.append_data(&mut header, path, contents.as_bytes())
            .unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

#[test]
fn isolated_install_stages_every_runner_and_retries_a_failed_download() {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    use codelab_core::{ErrorCode, InstallProgress};
    use sha2::{Digest, Sha256};

    let windows = cfg!(windows);
    let fixtures = HashMap::from([
        (
            "go",
            fixture_archive(
                &[(
                    if windows {
                        "go/bin/go.exe"
                    } else {
                        "go/bin/go"
                    },
                    "go test",
                )],
                windows,
            ),
        ),
        (
            "openjdk",
            fixture_archive(
                if windows {
                    &[("jdk/bin/java.exe", "java"), ("jdk/bin/javac.exe", "javac")]
                } else if cfg!(target_os = "macos") {
                    &[
                        ("jdk/Contents/Home/bin/java", "java"),
                        ("jdk/Contents/Home/bin/javac", "javac"),
                    ]
                } else {
                    &[("jdk/bin/java", "java"), ("jdk/bin/javac", "javac")]
                },
                windows,
            ),
        ),
        ("junit", b"JUnit 5".to_vec()),
        (
            "uv",
            fixture_archive(
                &[(if windows { "uv.exe" } else { "uv-release/uv" }, "uv")],
                windows,
            ),
        ),
        (
            "cpython",
            fixture_archive(
                &[(
                    if windows {
                        "python/python.exe"
                    } else {
                        "python/bin/python3.12"
                    },
                    "Python 3.12",
                )],
                false,
            ),
        ),
        (
            "pytest-",
            fixture_archive(&[("pytest/__main__.py", "pytest")], true),
        ),
        (
            "pytest_json_report-",
            fixture_archive(&[("pytest_jsonreport/plugin.py", "plugin")], true),
        ),
        (
            "pytest_metadata-",
            fixture_archive(&[("pytest_metadata/plugin.py", "metadata")], true),
        ),
        (
            "iniconfig-",
            fixture_archive(&[("iniconfig/__init__.py", "fixture")], true),
        ),
        (
            "packaging-",
            fixture_archive(&[("packaging/__init__.py", "fixture")], true),
        ),
        (
            "pluggy-",
            fixture_archive(&[("pluggy/__init__.py", "fixture")], true),
        ),
        (
            "pygments-",
            fixture_archive(&[("pygments/__init__.py", "fixture")], true),
        ),
        (
            "colorama-",
            fixture_archive(&[("colorama/__init__.py", "fixture")], true),
        ),
    ]);
    let checksums = fixtures
        .iter()
        .map(|(&key, bytes)| (key, format!("{:x}", Sha256::digest(bytes))))
        .collect();
    let requests = fixtures.len() + 2 - usize::from(!windows); // Java retries both artifacts.
    let http = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", http.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut first_junit = true;
        for stream in http.incoming().take(requests) {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(&stream);
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let key = request
                .split_whitespace()
                .nth(1)
                .unwrap()
                .trim_start_matches('/');
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = if key == "junit" && first_junit {
                first_junit = false;
                b"incomplete".as_slice()
            } else {
                fixtures[key].as_slice()
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        }
    });

    let dir = TempDir::new().unwrap();
    let app = App::open_with_runtime_adapters(
        AppConfig {
            data_dir: dir.path().to_path_buf(),
        },
        RuntimeAdapters {
            host: Arc::new(MissingToolchains),
            downloads: Arc::new(LoopbackInstaller { origin, checksums }),
        },
    )
    .unwrap();
    let events = app.subscribe();
    report(&app, AppCommand::RefreshPreflight);
    wait_for_report(&events, |report| report.checked);
    let install = |language| AppCommand::InstallManagedRuntime { language };

    report(&app, install(CourseLanguage::Java));
    let failed = wait_for_report(&events, |report| {
        matches!(
            language(report, CourseLanguage::Java).install,
            Some(InstallProgress::Failed { .. })
        )
    });
    let java = language(&failed, CourseLanguage::Java);
    assert!(java.can_install);
    assert!(java.managed.is_none());
    assert!(
        matches!(&java.install, Some(InstallProgress::Failed { error }) if error.code == ErrorCode::RuntimeIntegrityFailed)
    );

    for lang in [
        CourseLanguage::Java,
        CourseLanguage::Go,
        CourseLanguage::Python,
    ] {
        report(&app, install(lang));
    }
    let installed = wait_for_report(&events, |report| {
        [
            CourseLanguage::Java,
            CourseLanguage::Go,
            CourseLanguage::Python,
        ]
        .iter()
        .all(|lang| language(report, *lang).selected == Some(RuntimeSource::Managed))
    });
    for lang in [
        CourseLanguage::Java,
        CourseLanguage::Go,
        CourseLanguage::Python,
    ] {
        let row = language(&installed, lang);
        assert_eq!(row.status, ProbeStatus::Ok);
        assert_eq!(row.lsp.as_ref().unwrap().status, ProbeStatus::Missing);
        let managed = row.managed.as_ref().unwrap();
        assert!(managed.executable.starts_with(dir.path().join("runtimes")));
        assert!(managed.test_executor.is_file());
    }
    let java = language(&installed, CourseLanguage::Java)
        .managed
        .as_ref()
        .unwrap();
    assert_eq!(std::fs::read(&java.test_executor).unwrap(), b"JUnit 5");
    let python = language(&installed, CourseLanguage::Python)
        .managed
        .as_ref()
        .unwrap();
    assert!(python.uv.as_ref().unwrap().is_file());
    server.join().unwrap();
}

#[cfg(unix)]
#[test]
fn native_preflight_checks_version_and_times_out_a_stalled_executable() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    if let Some(data_dir) = std::env::var_os("CODELAB_PREFLIGHT_TEST_DATA") {
        let app = App::open(AppConfig {
            data_dir: data_dir.into(),
        })
        .unwrap();
        let events = app.subscribe();
        report(&app, AppCommand::RefreshPreflight);
        let checked = wait_for_report(&events, |report| report.checked);
        let java = language(&checked, CourseLanguage::Java);
        assert_eq!(java.system[0].value.as_deref(), Some("21.0.4"));
        assert_eq!(java.status, ProbeStatus::Ok);
        let go = language(&checked, CourseLanguage::Go);
        assert_eq!(go.status, ProbeStatus::Ok);
        let lsp = go.lsp.as_ref().unwrap();
        assert_eq!(lsp.status, ProbeStatus::Warn);
        assert_eq!(lsp.problem, Some(ProbeProblem::Failed));
        assert_eq!(checked.tools[1].problem, Some(ProbeProblem::TimedOut));
        return;
    }

    let dir = TempDir::new().unwrap();
    let bin = dir.path().join("bin");
    let jdk = dir.path().join("jdk");
    let java_bin = jdk.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::create_dir_all(&java_bin).unwrap();
    for (path, script) in [
        (bin.join("go"), "printf 'go version go1.22.0\n'"),
        (bin.join("gopls"), "exit 1"),
        (bin.join("curl"), "exec /bin/sleep 15"),
        (bin.join("java"), "exit 0"),
        (bin.join("javac"), "printf 'javac 17.0.1\n' >&2"),
        (java_bin.join("java"), "exit 0"),
        (java_bin.join("javac"), "printf 'javac 21.0.4\n' >&2"),
    ] {
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native_preflight_checks_version_and_times_out_a_stalled_executable",
        ])
        .env("CODELAB_PREFLIGHT_TEST_DATA", dir.path().join("data"))
        .env("PATH", bin)
        .env("JAVA_HOME", jdk)
        .env("GOPROXY", "direct")
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}
