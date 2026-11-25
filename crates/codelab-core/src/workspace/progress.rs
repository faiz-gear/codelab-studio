use crate::{
    Chapter, ChapterKind, ChapterProgress, CheckKind, CheckStatus, CompletionState, LoadedCourse,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Progress {
    pub chapters: BTreeMap<String, ChapterProgress>,
}
impl Progress {
    pub fn valid(&self) -> bool {
        self.chapters.iter().all(|(id, chapter)| {
            let mut ids = BTreeSet::new();
            !id.trim().is_empty()
                && chapter.self_checks.keys().all(|id| !id.trim().is_empty())
                && chapter.last_check_results.iter().all(|result| {
                    let mut tests = BTreeSet::new();
                    !result.check_id.trim().is_empty()
                        && ids.insert(&result.check_id)
                        && (result.status != CheckStatus::Passed
                            || (result.error.is_none() && result.diffs.is_empty()))
                        && result
                            .test_cases
                            .iter()
                            .all(|test| !test.name.trim().is_empty() && tests.insert(&test.name))
                })
        })
    }
    pub fn completed_for(&self, course: &LoadedCourse) -> u32 {
        course
            .chapters
            .iter()
            .filter(|chapter| {
                chapter.kind == ChapterKind::Exercise
                    && self
                        .chapters
                        .get(&chapter.chapter_id)
                        .is_some_and(|p| p.completion_state == CompletionState::Completed)
            })
            .count() as u32
    }
    pub fn update_completion(&mut self, chapter: &Chapter, manual: bool) {
        let progress = self.chapters.entry(chapter.chapter_id.clone()).or_default();
        if progress.completion_state == CompletionState::Completed {
            return;
        }
        let checks = chapter
            .three_step
            .as_ref()
            .map(|s| s.step3_checks.as_slice())
            .unwrap_or_default();
        let automatic: Vec<_> = checks
            .iter()
            .filter(|c| c.kind != CheckKind::SelfCheck)
            .collect();
        let passed = if automatic.is_empty() {
            manual
        } else {
            automatic.iter().all(|c| {
                progress
                    .last_check_results
                    .iter()
                    .any(|r| r.check_id == c.id && r.status == CheckStatus::Passed)
            })
        };
        let required = checks
            .iter()
            .filter(|c| c.kind == CheckKind::SelfCheck && c.required)
            .all(|c| progress.self_checks.get(&c.id) == Some(&true));
        progress.completion_state = if passed && required {
            CompletionState::Completed
        } else {
            CompletionState::InProgress
        };
    }
}
