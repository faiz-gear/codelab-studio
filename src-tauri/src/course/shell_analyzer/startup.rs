use super::{
    count_leading_env_assignments, extract_cd_target_arg, extract_shell_segments,
    tokenize_shell_words, words_match_prefix,
};

pub(crate) fn block_starts_with_root_reset_cd(script: &str) -> bool {
    let raw_segments = extract_shell_segments(script);
    let Some(first_raw) = raw_segments.first() else {
        return false;
    };
    let words = tokenize_shell_words(first_raw);
    let effective_start = count_leading_env_assignments(&words);
    let effective = &words[effective_start..];
    if !effective
        .first()
        .is_some_and(|word| word.eq_ignore_ascii_case("cd"))
    {
        return false;
    }

    match extract_cd_target_arg(&effective[1..]) {
        None => true,
        Some(raw_arg) => {
            let trimmed = raw_arg.trim();
            trimmed.starts_with('/')
                || trimmed.starts_with('~')
                || trimmed.starts_with('$')
                || trimmed.starts_with('\\')
                || (trimmed.len() >= 2
                    && trimmed.as_bytes()[0].is_ascii_alphabetic()
                    && trimmed.as_bytes()[1] == b':')
        }
    }
}

pub(crate) fn block_starts_with_root_reset_or_init(script: &str) -> bool {
    if block_starts_with_root_reset_cd(script) {
        return true;
    }
    let raw_segments = extract_shell_segments(script);
    let Some(first_raw) = raw_segments.first() else {
        return false;
    };
    let words = tokenize_shell_words(first_raw);
    let effective_start = count_leading_env_assignments(&words);
    let effective = &words[effective_start..];
    effective
        .first()
        .is_some_and(|word| word.eq_ignore_ascii_case("mkdir"))
        || words_match_prefix(effective, &["gf", "init"])
}

#[cfg(test)]
mod tests {
    use super::{block_starts_with_root_reset_cd, block_starts_with_root_reset_or_init};

    #[test]
    fn root_reset_cd_detects_absolute_targets_without_resetting_relative_cd() {
        assert!(block_starts_with_root_reset_cd(
            "cd /abs/path && gf init task-api-gf && cd task-api-gf && go run main.go"
        ));
        assert!(block_starts_with_root_reset_cd("cd ~/work && go run ."));
        assert!(block_starts_with_root_reset_cd("cd"));
        assert!(block_starts_with_root_reset_cd("cd C:\\work && go run ."));
        assert!(!block_starts_with_root_reset_cd(
            "cd .. && gf init task-api-gf"
        ));
        assert!(!block_starts_with_root_reset_cd("cd sub && go run ."));
        assert!(!block_starts_with_root_reset_cd("go test ./..."));
    }

    #[test]
    fn root_reset_or_init_detects_project_initializers() {
        assert!(block_starts_with_root_reset_or_init(
            "mkdir task-api\ncd task-api\ngo mod init example.com/task-api"
        ));
        assert!(block_starts_with_root_reset_or_init("MKDIR -p task-api"));
        assert!(block_starts_with_root_reset_or_init(
            "gf init task-api-gf -g example.com/task-api-gf"
        ));
        assert!(block_starts_with_root_reset_or_init("GF INIT task-api-gf"));
        assert!(block_starts_with_root_reset_or_init(
            "cd /Users/dev/work && gf init task-api-gf"
        ));
        assert!(!block_starts_with_root_reset_or_init("go test -race ./..."));
        assert!(!block_starts_with_root_reset_or_init(
            "go get modernc.org/sqlite@latest"
        ));
        assert!(!block_starts_with_root_reset_or_init("gf run main.go"));
    }
}
