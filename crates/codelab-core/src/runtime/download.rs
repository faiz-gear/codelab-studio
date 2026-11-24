use std::io::{Read, Write};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::{Host, Url};

use crate::{DomainError, ErrorCode};

#[derive(Debug, Deserialize)]
pub struct RuntimeArtifact {
    pub url: String,
    pub sha256: String,
}

impl RuntimeArtifact {
    pub(super) fn filename(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or_default()
    }
}

pub trait RuntimeDownloadAdapter: Send + Sync {
    fn download(
        &self,
        artifact: &RuntimeArtifact,
        destination: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DomainError>;
}

pub struct HttpRuntimeDownloader;

impl RuntimeDownloadAdapter for HttpRuntimeDownloader {
    fn download(
        &self,
        artifact: &RuntimeArtifact,
        destination: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DomainError> {
        let failure = || DomainError::new(ErrorCode::RuntimeDownloadFailed);
        let url = Url::parse(&artifact.url).map_err(|_| failure())?;
        let loopback = match url.host() {
            Some(Host::Ipv4(addr)) => addr.is_loopback(),
            Some(Host::Ipv6(addr)) => addr.is_loopback(),
            _ => false,
        };

        let mut client = Client::builder()
            .https_only(!loopback)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(45))
            .user_agent("CodeLab Studio");
        if loopback {
            client = client
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none());
        }
        let mut response = client
            .build()
            .and_then(|client| client.get(url).send())
            .map_err(|_| failure())?;
        if !response.status().is_success() {
            return Err(failure().with_detail("status", response.status().as_str()));
        }

        const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
        let total = response.content_length();
        let mut hash = Sha256::new();
        let mut received = 0u64;
        let mut last_update = None;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = response.read(&mut buffer).map_err(|_| failure())?;
            if count == 0 {
                break;
            }
            received += count as u64;
            if received > MAX_DOWNLOAD_BYTES {
                return Err(failure());
            }
            let chunk = &buffer[..count];
            destination
                .write_all(chunk)
                .map_err(|_| DomainError::new(ErrorCode::RuntimeInstallFailed))?;
            hash.update(chunk);

            let now = Instant::now();
            if last_update.is_none_or(|time| now.duration_since(time) >= Duration::from_millis(125))
                || Some(received) == total
            {
                progress(received, total);
                last_update = Some(now);
            }
        }
        if format!("{:x}", hash.finalize()) != artifact.sha256 {
            return Err(DomainError::new(ErrorCode::RuntimeIntegrityFailed));
        }
        Ok(())
    }
}
