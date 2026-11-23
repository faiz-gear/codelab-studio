use crate::domain::error::{AppError, ErrorCode};

pub fn validate_git_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(git_validation_error("git repository URL must not be empty"));
    }
    if trimmed.starts_with('-') {
        return Err(git_validation_error(
            "git repository URL must not start with '-'",
        ));
    }
    if trimmed.contains('\0')
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(git_validation_error(
            "git repository URL contains disallowed control or whitespace characters",
        ));
    }

    for scheme in ["https://", "http://", "ssh://"] {
        if let Some(rest) = strip_prefix_ignore_ascii_case(trimmed, scheme) {
            validate_url_authority_and_path(rest, scheme == "ssh://")?;
            return Ok(trimmed.to_owned());
        }
    }

    if let Some(rest) = trimmed.strip_prefix("git@") {
        let Some((host, path)) = rest.split_once(':') else {
            return Err(git_validation_error(
                "SCP-style git URL must use git@<host>:<path>",
            ));
        };
        if !is_valid_host_token(host) {
            return Err(git_validation_error(
                "SCP-style git URL contains an invalid host",
            ));
        }
        if path.is_empty()
            || path.starts_with('/')
            || path.contains("://")
            || path.contains('\\')
            || path.trim_matches('/').is_empty()
        {
            return Err(git_validation_error(
                "SCP-style git URL contains an invalid repository path",
            ));
        }
        return Ok(trimmed.to_owned());
    }

    Err(git_validation_error(
        "only https://, http://, ssh://, and git@<host>:<path> URLs are permitted",
    ))
}

pub fn validate_git_ref(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(git_validation_error("git ref must not be empty"));
    }
    if trimmed.starts_with('-') {
        return Err(git_validation_error("git ref must not start with '-'"));
    }
    if trimmed.contains('\0')
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(git_validation_error(
            "git ref contains disallowed control or whitespace characters",
        ));
    }
    if trimmed == "@"
        || trimmed.starts_with('/')
        || trimmed.ends_with('/')
        || trimmed.ends_with('.')
        || trimmed.ends_with(".lock")
        || trimmed.contains("..")
        || trimmed.contains("//")
        || trimmed.contains("@{")
        || trimmed
            .chars()
            .any(|ch| matches!(ch, '~' | '^' | ':' | '\\' | '?' | '*' | '['))
    {
        return Err(git_validation_error("git ref contains invalid syntax"));
    }

    for segment in trimmed.split('/') {
        if segment.is_empty() || segment.starts_with('.') || segment.ends_with(".lock") {
            return Err(git_validation_error(
                "git ref contains an invalid path segment",
            ));
        }
    }

    Ok(trimmed.to_owned())
}

fn validate_url_authority_and_path(rest: &str, allow_ssh_user: bool) -> Result<(), AppError> {
    if rest.contains('?') || rest.contains('#') {
        return Err(git_validation_error(
            "git URL query strings and fragments are not permitted",
        ));
    }

    let Some((authority, path)) = rest.split_once('/') else {
        return Err(git_validation_error(
            "git URL must include both host and repository path",
        ));
    };
    if authority.is_empty()
        || path.is_empty()
        || path.contains('\\')
        || path.trim_matches('/').is_empty()
    {
        return Err(git_validation_error(
            "git URL must include a valid host and non-empty repository path",
        ));
    }

    let host_port = match authority.rsplit_once('@') {
        Some((username, host)) if allow_ssh_user && username == "git" => host,
        Some(_) => {
            return Err(git_validation_error(
                "credential-bearing git URL userinfo is not permitted",
            ));
        }
        None => authority,
    };
    if host_port.is_empty() || host_port.starts_with('-') {
        return Err(git_validation_error("git URL contains an invalid host"));
    }

    if let Some(after_open) = host_port.strip_prefix('[') {
        let Some((ipv6, remainder)) = after_open.split_once(']') else {
            return Err(git_validation_error(
                "git URL contains an unclosed IPv6 host",
            ));
        };
        if ipv6.is_empty()
            || !ipv6
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.')
        {
            return Err(git_validation_error(
                "git URL contains an invalid IPv6 host",
            ));
        }
        if !remainder.is_empty() {
            let Some(port) = remainder.strip_prefix(':') else {
                return Err(git_validation_error("git URL contains an invalid port"));
            };
            validate_port_str(port)?;
        }
        return Ok(());
    }

    let (host, port_opt) = match host_port.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host_port, None),
    };
    if !is_valid_host_token(host) {
        return Err(git_validation_error("git URL contains an invalid host"));
    }
    if let Some(port) = port_opt {
        validate_port_str(port)?;
    }
    Ok(())
}

fn is_valid_host_token(host: &str) -> bool {
    !host.is_empty()
        && !host.starts_with('-')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

fn validate_port_str(port: &str) -> Result<(), AppError> {
    if !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|value| value > 0)
    {
        Ok(())
    } else {
        Err(git_validation_error("git URL contains an invalid port"))
    }
}

fn strip_prefix_ignore_ascii_case<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    input
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &input[prefix.len()..])
}

pub(super) fn git_validation_error(detail: &str) -> AppError {
    AppError::new(ErrorCode::SchemaValidationFailed, "error.gitImportFailed")
        .with_param("detail", detail)
        .with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::{validate_git_ref, validate_git_url};
    use crate::domain::error::ErrorCode;

    #[test]
    fn validates_protocols_and_rejects_credentials() {
        for valid in [
            "https://github.com/org/course-pack.git",
            "http://127.0.0.1:8080/repo.git",
            "http://[::1]:9000/course.git",
            "ssh://git@github.com/org/course.git",
            "git@github.com:org/course.git",
        ] {
            assert!(
                validate_git_url(valid).is_ok(),
                "expected '{valid}' to be accepted"
            );
        }

        for invalid in [
            "",
            "   ",
            "file:///tmp/repo",
            "ext::sh -c id",
            "git://github.com/org/repo.git",
            "ftp://example.com/repo.git",
            "/tmp/local/repo",
            "./relative/repo",
            "--upload-pack=evil",
            "https://github.com/org/repo.git --config=x",
            "https://github.com/",
            "https://user:token@github.com/org/repo.git",
            "https://token@github.com/org/repo.git",
            "ssh://git:token@github.com/org/repo.git",
            "https://github.com/org/repo.git?access_token=secret",
            "https://github.com/org/repo.git#access_token=secret",
            "git@github.com:/abs/path",
            "git@-badhost:org/repo.git",
        ] {
            let err = validate_git_url(invalid).expect_err("invalid URL must fail");
            assert_eq!(err.code, ErrorCode::SchemaValidationFailed);
            assert_eq!(err.message_key, "error.gitImportFailed");
        }
    }

    #[test]
    fn validates_refs_without_accepting_option_injection() {
        for valid in ["main", "v1.0.0", "release/2026-q4", "feat_u2"] {
            assert!(validate_git_ref(valid).is_ok());
        }

        for invalid in [
            "",
            "   ",
            "--all",
            "-b",
            "main..dev",
            "main^1",
            "main~2",
            "bad:ref",
            "bad ref",
            "bad@{0}",
            "branch.lock",
            "/leading",
            "trailing/",
            "a/.hidden",
        ] {
            let err = validate_git_ref(invalid).expect_err("invalid ref must fail");
            assert_eq!(err.code, ErrorCode::SchemaValidationFailed);
            assert_eq!(err.message_key, "error.gitImportFailed");
        }
    }
}
