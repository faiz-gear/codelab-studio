use std::io::Write;

use crate::domain::error::AppError;
use crate::domain::workspace::{BootstrapState, Locale};
use crate::session::AppState;

pub fn detect_system_locale(raw: Option<&str>) -> Locale {
    let Some(trimmed) = raw.map(str::trim) else {
        return Locale::EnUs;
    };
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("zh"))
    {
        Locale::ZhCn
    } else {
        Locale::EnUs
    }
}

fn locale_tag(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "zh-CN",
        Locale::EnUs => "en-US",
    }
}

#[tauri::command]
pub fn app_bootstrap(state: tauri::State<'_, AppState>) -> Result<BootstrapState, AppError> {
    let detected = sys_locale::get_locale();
    let locale = detect_system_locale(detected.as_deref());
    let bootstrap = state.build_bootstrap_state(locale)?;

    if std::env::var("CODELAB_STARTUP_PROBE").as_deref() == Ok("1") {
        let startup_ms = state.elapsed_startup_ms();
        let locale_str = locale_tag(bootstrap.config.locale);
        let epoch = bootstrap.epoch.0;
        let builtin_courses = serde_json::json!(bootstrap.builtin_courses);
        println!(
            r#"CODELAB_BOOTSTRAP_READY {{"startupMs":{startup_ms},"locale":"{locale_str}","epoch":{epoch},"builtinCourses":{builtin_courses}}}"#
        );
        let _ = std::io::stdout().flush();
    }

    Ok(bootstrap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::CodelabHomePaths;

    #[test]
    fn detect_system_locale_maps_zh_prefixes_case_insensitively() {
        for raw in ["zh", "zh-CN", "zh_TW", "ZH-Hans-SG", "  zh-hk  "] {
            assert_eq!(
                detect_system_locale(Some(raw)),
                Locale::ZhCn,
                "expected '{raw}' to resolve to zh-CN"
            );
        }

        for raw in ["", "z", "en", "en-US", "ja-JP", "中文"] {
            assert_eq!(
                detect_system_locale(Some(raw)),
                Locale::EnUs,
                "expected '{raw}' to resolve to en-US"
            );
        }

        assert_eq!(detect_system_locale(None), Locale::EnUs);
    }

    #[test]
    fn bootstrap_state_serializes_expected_contract() {
        let temp_home = tempfile::tempdir().expect("temp home");
        let state =
            AppState::with_home_paths(CodelabHomePaths::from_root(temp_home.path().to_path_buf()));
        let locale = detect_system_locale(Some("zh-CN"));
        let bootstrap = state.build_bootstrap_state(locale).unwrap();
        let value = serde_json::to_value(&bootstrap).unwrap();

        assert_eq!(value["epoch"], 0);
        assert_eq!(value["config"]["schemaVersion"], 1);
        assert_eq!(value["config"]["locale"], "zh-CN");
        assert_eq!(value["recentWorkspaces"], serde_json::json!([]));
        assert_eq!(value["builtinCourses"], serde_json::json!([]));
    }
}
