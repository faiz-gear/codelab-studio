use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::domain::ai::TeachingMode;
use crate::domain::events::StorageCorruptedEvent;
use crate::domain::workspace::{
    AccentPreset, ExternalIdeKind, GlobalConfigPatch, GlobalConfigView, Locale,
    RecentWorkspaceItem, StorageSchemaVersion, ThemeMode,
};
use crate::platform::{CodelabHomePaths, DirectoryFileLock};
use crate::storage::atomic_json::{
    cleanup_stale_tmp_files, AtomicJsonFile, LoadOutcome, StorageError,
};

pub const MAX_RECENT_WORKSPACES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiProtocolKind {
    Byok,
    Acp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceConfig {
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub code_font_size_px: u16,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Light,
            accent: AccentPreset::Neutral,
            code_font_size_px: 13,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfig {
    pub external_ide: ExternalIdeKind,
    pub lsp_enabled: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            external_ide: ExternalIdeKind::Vscode,
            lsp_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByokProfileRef {
    pub profile_id: String,
    pub label: String,
    pub provider: String,
    pub base_url: Option<String>,
    pub model: String,
    pub keychain_account: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpAgentEntry {
    pub agent_id: String,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGlobalConfig {
    pub active_protocol: AiProtocolKind,
    pub default_teaching_mode: TeachingMode,
    #[serde(default)]
    pub byok_profiles: Vec<ByokProfileRef>,
    #[serde(default)]
    pub acp_agents: Vec<AcpAgentEntry>,
}

impl Default for AiGlobalConfig {
    fn default() -> Self {
        Self {
            active_protocol: AiProtocolKind::Byok,
            default_teaching_mode: TeachingMode::Socratic,
            byok_profiles: Vec::new(),
            acp_agents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyConfig {
    pub opt_in_anonymous_telemetry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfigFile {
    pub schema_version: StorageSchemaVersion,
    pub locale: Locale,
    pub appearance: AppearanceConfig,
    pub editor: EditorConfig,
    pub ai: AiGlobalConfig,
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub recent_workspaces: Vec<RecentWorkspaceItem>,
}

impl GlobalConfigFile {
    pub fn default_with_locale(locale: Locale) -> Self {
        Self {
            schema_version: StorageSchemaVersion::V1,
            locale,
            appearance: AppearanceConfig::default(),
            editor: EditorConfig::default(),
            ai: AiGlobalConfig::default(),
            privacy: PrivacyConfig {
                opt_in_anonymous_telemetry: false,
            },
            recent_workspaces: Vec::new(),
        }
    }

    pub fn to_view(&self) -> GlobalConfigView {
        GlobalConfigView {
            schema_version: self.schema_version,
            locale: self.locale,
            theme: self.appearance.theme,
            accent: self.appearance.accent,
            code_font_size_px: self.appearance.code_font_size_px,
            external_ide: self.editor.external_ide,
            lsp_enabled: self.editor.lsp_enabled,
            default_teaching_mode: self.ai.default_teaching_mode,
            opt_in_anonymous_telemetry: self.privacy.opt_in_anonymous_telemetry,
        }
    }

    pub fn apply_patch(&mut self, patch: GlobalConfigPatch) {
        if let Some(locale) = patch.locale {
            self.locale = locale;
        }
        if let Some(theme) = patch.theme {
            self.appearance.theme = theme;
        }
        if let Some(accent) = patch.accent {
            self.appearance.accent = accent;
        }
        if let Some(code_font_size_px) = patch.code_font_size_px {
            self.appearance.code_font_size_px = code_font_size_px.clamp(10, 28);
        }
        if let Some(external_ide) = patch.external_ide {
            self.editor.external_ide = external_ide;
        }
        if let Some(lsp_enabled) = patch.lsp_enabled {
            self.editor.lsp_enabled = lsp_enabled;
        }
        if let Some(default_teaching_mode) = patch.default_teaching_mode {
            self.ai.default_teaching_mode = default_teaching_mode;
        }
        if let Some(opt_in_anonymous_telemetry) = patch.opt_in_anonymous_telemetry {
            self.privacy.opt_in_anonymous_telemetry = opt_in_anonymous_telemetry;
        }
    }

    pub fn record_recent_workspace(&mut self, item: RecentWorkspaceItem) {
        self.recent_workspaces.retain(|existing| {
            existing.workspace_id != item.workspace_id
                && existing.workspace_root != item.workspace_root
        });
        self.recent_workspaces.insert(0, item);
        if self.recent_workspaces.len() > MAX_RECENT_WORKSPACES {
            self.recent_workspaces.truncate(MAX_RECENT_WORKSPACES);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPackEntry {
    pub source_key: String,
    pub content_hash: String,
    pub trusted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPacksFile {
    pub schema_version: StorageSchemaVersion,
    #[serde(default)]
    pub packs: Vec<TrustedPackEntry>,
}

impl Default for TrustedPacksFile {
    fn default() -> Self {
        Self {
            schema_version: StorageSchemaVersion::V1,
            packs: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct GlobalStore {
    paths: CodelabHomePaths,
    config_file: AtomicJsonFile<GlobalConfigFile>,
    trusted_packs_file: AtomicJsonFile<TrustedPacksFile>,
}

impl GlobalStore {
    pub fn new(paths: CodelabHomePaths) -> Self {
        if let Ok(_lock) = DirectoryFileLock::lock_blocking(&paths.lock_file()) {
            let _ = cleanup_stale_tmp_files(paths.root());
        }
        let config_file = AtomicJsonFile::new(paths.config_path());
        let trusted_packs_file = AtomicJsonFile::new(paths.trusted_packs_path());
        Self {
            paths,
            config_file,
            trusted_packs_file,
        }
    }

    pub fn paths(&self) -> &CodelabHomePaths {
        &self.paths
    }

    fn acquire_home_lock(&self) -> Result<DirectoryFileLock, StorageError> {
        let lock_path = self.paths.lock_file();
        DirectoryFileLock::lock_blocking(&lock_path).map_err(|source| StorageError::Io {
            path: lock_path,
            source,
        })
    }

    pub fn load_or_init_config(
        &self,
        default_locale: Locale,
    ) -> Result<LoadOutcome<GlobalConfigFile>, StorageError> {
        let _home_lock = self.acquire_home_lock()?;
        self.config_file
            .load_or_init(|| GlobalConfigFile::default_with_locale(default_locale))
    }

    pub fn update_config(
        &self,
        default_locale: Locale,
        patch: GlobalConfigPatch,
    ) -> Result<(GlobalConfigView, Option<StorageCorruptedEvent>), StorageError> {
        let _home_lock = self.acquire_home_lock()?;
        self.config_file.load_or_init_and_update(
            || GlobalConfigFile::default_with_locale(default_locale),
            |cfg| {
                cfg.apply_patch(patch);
                Ok(cfg.to_view())
            },
        )
    }

    pub fn record_recent_workspace(
        &self,
        default_locale: Locale,
        item: RecentWorkspaceItem,
    ) -> Result<Vec<RecentWorkspaceItem>, StorageError> {
        let _home_lock = self.acquire_home_lock()?;
        let (recents, _) = self.config_file.load_or_init_and_update(
            || GlobalConfigFile::default_with_locale(default_locale),
            |cfg| {
                cfg.record_recent_workspace(item);
                Ok(cfg.recent_workspaces.clone())
            },
        )?;
        Ok(recents)
    }

    pub fn is_pack_trusted(
        &self,
        source_key: &str,
        content_hash: &str,
    ) -> Result<bool, StorageError> {
        let _home_lock = self.acquire_home_lock()?;
        let outcome = self
            .trusted_packs_file
            .load_or_init(TrustedPacksFile::default)?;
        Ok(outcome
            .value()
            .packs
            .iter()
            .any(|p| p.source_key == source_key && p.content_hash == content_hash))
    }

    pub fn trust_pack(
        &self,
        source_key: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Result<TrustedPackEntry, StorageError> {
        let _home_lock = self.acquire_home_lock()?;
        let source_key = source_key.into();
        let content_hash = content_hash.into();
        let entry = TrustedPackEntry {
            source_key: source_key.clone(),
            content_hash: content_hash.clone(),
            trusted_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };
        let (saved, _) =
            self.trusted_packs_file
                .load_or_init_and_update(TrustedPacksFile::default, |file| {
                    if let Some(existing) = file
                        .packs
                        .iter()
                        .find(|p| p.source_key == source_key && p.content_hash == content_hash)
                    {
                        return Ok(existing.clone());
                    }
                    file.packs.retain(|p| p.source_key != source_key);
                    file.packs.push(entry.clone());
                    Ok(entry)
                })?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::domain::runtime::Language;

    fn recent_item(id: &str, root: &str) -> RecentWorkspaceItem {
        RecentWorkspaceItem {
            workspace_id: id.to_owned(),
            workspace_root: root.to_owned(),
            course_id: "go-goframe-practice".to_owned(),
            course_title: "Go & GoFrame".to_owned(),
            language: Language::Go,
            completed_chapters: 1,
            total_chapters: 10,
            last_opened_at: "2026-11-23T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn global_config_persists_patches_and_keeps_zero_api_key_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = GlobalStore::new(CodelabHomePaths::from_root(dir.path().to_path_buf()));

        let initial = store
            .load_or_init_config(Locale::ZhCn)
            .expect("init config");
        assert!(matches!(initial, LoadOutcome::CreatedDefault(_)));
        assert_eq!(initial.value().locale, Locale::ZhCn);
        assert!(!initial.value().privacy.opt_in_anonymous_telemetry);

        let (patched, corruption) = store
            .update_config(
                Locale::ZhCn,
                GlobalConfigPatch {
                    locale: Some(Locale::EnUs),
                    theme: Some(ThemeMode::Dark),
                    accent: Some(AccentPreset::Violet),
                    code_font_size_px: Some(15),
                    external_ide: Some(ExternalIdeKind::Cursor),
                    lsp_enabled: Some(false),
                    default_teaching_mode: Some(TeachingMode::Direct),
                    opt_in_anonymous_telemetry: Some(false),
                },
            )
            .expect("patch config");
        assert!(corruption.is_none());
        assert_eq!(patched.locale, Locale::EnUs);
        assert_eq!(patched.theme, ThemeMode::Dark);
        assert_eq!(patched.accent, AccentPreset::Violet);
        assert_eq!(patched.code_font_size_px, 15);
        assert_eq!(patched.external_ide, ExternalIdeKind::Cursor);
        assert!(!patched.lsp_enabled);
        assert_eq!(patched.default_teaching_mode, TeachingMode::Direct);

        let raw = fs::read_to_string(dir.path().join("config.json")).expect("read config.json");
        assert!(!raw.contains("apiKey"));
        assert!(!raw.contains("api_key"));

        let reloaded = store
            .load_or_init_config(Locale::ZhCn)
            .expect("reload config");
        assert!(matches!(reloaded, LoadOutcome::Loaded(_)));
        assert_eq!(reloaded.value().to_view(), patched);
    }

    #[test]
    fn recent_workspaces_deduplicates_moves_to_front_and_caps_at_20() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = GlobalStore::new(CodelabHomePaths::from_root(dir.path().to_path_buf()));

        for idx in 0..25 {
            store
                .record_recent_workspace(
                    Locale::EnUs,
                    recent_item(&format!("ws_{idx}"), &format!("/tmp/ws-{idx}")),
                )
                .expect("record recent");
        }

        let updated = store
            .record_recent_workspace(Locale::EnUs, recent_item("ws_10", "/tmp/ws-10"))
            .expect("re-record ws_10");
        assert_eq!(updated.len(), MAX_RECENT_WORKSPACES);
        assert_eq!(updated[0].workspace_id, "ws_10");
        assert_eq!(
            updated
                .iter()
                .filter(|item| item.workspace_id == "ws_10")
                .count(),
            1
        );
    }

    #[test]
    fn global_config_recovers_from_bak_when_truncated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = GlobalStore::new(CodelabHomePaths::from_root(dir.path().to_path_buf()));

        let _ = store.load_or_init_config(Locale::ZhCn).unwrap();
        let _ = store
            .update_config(
                Locale::ZhCn,
                GlobalConfigPatch {
                    theme: Some(ThemeMode::Dark),
                    ..Default::default()
                },
            )
            .unwrap();
        let _ = store
            .update_config(
                Locale::ZhCn,
                GlobalConfigPatch {
                    accent: Some(AccentPreset::Blue),
                    ..Default::default()
                },
            )
            .unwrap();

        fs::write(dir.path().join("config.json"), b"{\"schemaVersion\": 1,")
            .expect("truncate config");

        let recovered = store
            .load_or_init_config(Locale::EnUs)
            .expect("recover config");
        assert!(matches!(
            recovered,
            LoadOutcome::RecoveredFromBackup { ref value, .. }
                if value.appearance.theme == ThemeMode::Dark && value.locale == Locale::ZhCn
        ));
    }

    #[test]
    fn trusted_packs_trust_is_idempotent_and_updates_hash_on_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = GlobalStore::new(CodelabHomePaths::from_root(dir.path().to_path_buf()));

        assert!(!store
            .is_pack_trusted("git:https://example.com/a", "sha256:1")
            .unwrap());
        let first = store
            .trust_pack("git:https://example.com/a", "sha256:1")
            .unwrap();
        let second = store
            .trust_pack("git:https://example.com/a", "sha256:1")
            .unwrap();
        assert_eq!(first, second);
        assert!(store
            .is_pack_trusted("git:https://example.com/a", "sha256:1")
            .unwrap());
        assert!(!store
            .is_pack_trusted("git:https://example.com/a", "sha256:2")
            .unwrap());

        store
            .trust_pack("git:https://example.com/a", "sha256:2")
            .unwrap();
        assert!(store
            .is_pack_trusted("git:https://example.com/a", "sha256:2")
            .unwrap());
        assert!(!store
            .is_pack_trusted("git:https://example.com/a", "sha256:1")
            .unwrap());
    }
}
