use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use crate::domain::ai::TeachingMode;
use crate::domain::ids::SessionEpoch;
use crate::domain::metrics::MetricEvent;
use crate::domain::workspace::{
    AccentPreset, BootstrapState, ExternalIdeKind, GlobalConfigView, Locale, StorageSchemaVersion,
    ThemeMode,
};

pub const PENDING_METRICS_CAPACITY: usize = 256;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ActiveSessionSlot;

#[derive(Debug)]
pub struct AppState {
    started_at: Instant,
    current_epoch: AtomicU64,
    app_launched_recorded: AtomicBool,
    pending_metrics: Mutex<VecDeque<MetricEvent>>,
    active_session: Mutex<Option<ActiveSessionSlot>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            current_epoch: AtomicU64::new(0),
            app_launched_recorded: AtomicBool::new(false),
            pending_metrics: Mutex::new(VecDeque::with_capacity(PENDING_METRICS_CAPACITY)),
            active_session: Mutex::new(None),
        }
    }

    pub fn elapsed_startup_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }

    pub fn current_epoch(&self) -> SessionEpoch {
        SessionEpoch(self.current_epoch.load(Ordering::Acquire))
    }

    pub fn push_pending_metric(&self, event: MetricEvent) {
        let mut queue = self
            .pending_metrics
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if queue.len() >= PENDING_METRICS_CAPACITY {
            queue.pop_front();
        }
        queue.push_back(event);
    }

    pub fn drain_pending_metrics(&self) -> Vec<MetricEvent> {
        let mut queue = self
            .pending_metrics
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        queue.drain(..).collect()
    }

    pub fn drain_active_session_for_shutdown(&self) {
        let mut slot = self
            .active_session
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let _ = slot.take();
    }

    pub fn build_bootstrap_state(&self, locale: Locale) -> BootstrapState {
        if !self.app_launched_recorded.swap(true, Ordering::AcqRel) {
            self.push_pending_metric(MetricEvent::AppLaunched { locale });
        }

        BootstrapState {
            epoch: self.current_epoch(),
            config: GlobalConfigView {
                schema_version: StorageSchemaVersion::V1,
                locale,
                theme: ThemeMode::Light,
                accent: AccentPreset::Neutral,
                code_font_size_px: 13,
                external_ide: ExternalIdeKind::Vscode,
                lsp_enabled: true,
                default_teaching_mode: TeachingMode::Socratic,
                opt_in_anonymous_telemetry: false,
            },
            recent_workspaces: Vec::new(),
            builtin_courses: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_epoch_is_zero() {
        let state = AppState::new();
        assert_eq!(state.current_epoch(), SessionEpoch(0));
    }

    #[test]
    fn build_bootstrap_state_records_app_launched_once_idempotently() {
        let state = AppState::new();

        let first = state.build_bootstrap_state(Locale::ZhCn);
        let second = state.build_bootstrap_state(Locale::EnUs);

        assert_eq!(first.epoch, SessionEpoch(0));
        assert_eq!(first.config.schema_version, StorageSchemaVersion::V1);
        assert_eq!(first.config.locale, Locale::ZhCn);
        assert_eq!(first.config.theme, ThemeMode::Light);
        assert_eq!(first.config.accent, AccentPreset::Neutral);
        assert_eq!(first.config.default_teaching_mode, TeachingMode::Socratic);
        assert!(!first.config.opt_in_anonymous_telemetry);
        assert!(first.recent_workspaces.is_empty());
        assert!(first.builtin_courses.is_empty());

        assert_eq!(second.epoch, SessionEpoch(0));
        assert_eq!(second.config.locale, Locale::EnUs);

        let drained = state.drain_pending_metrics();
        assert_eq!(
            drained,
            vec![MetricEvent::AppLaunched {
                locale: Locale::ZhCn
            }]
        );
        assert!(state.drain_pending_metrics().is_empty());
    }

    #[test]
    fn pending_metrics_queue_is_bounded_and_drops_oldest() {
        let state = AppState::new();
        let total = (PENDING_METRICS_CAPACITY + 44) as u32;

        for chapter_index in 0..total {
            state.push_pending_metric(MetricEvent::VerifyChapterClicked { chapter_index });
        }

        let drained = state.drain_pending_metrics();
        assert_eq!(drained.len(), PENDING_METRICS_CAPACITY);
        assert_eq!(
            drained.first(),
            Some(&MetricEvent::VerifyChapterClicked { chapter_index: 44 })
        );
        assert_eq!(
            drained.last(),
            Some(&MetricEvent::VerifyChapterClicked {
                chapter_index: total - 1
            })
        );
        assert!(state.drain_pending_metrics().is_empty());
    }

    #[test]
    fn drain_active_session_for_shutdown_clears_slot() {
        let state = AppState::new();
        {
            let mut slot = state
                .active_session
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            *slot = Some(ActiveSessionSlot);
        }

        state.drain_active_session_for_shutdown();

        let slot = state
            .active_session
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert!(slot.is_none());
    }
}
