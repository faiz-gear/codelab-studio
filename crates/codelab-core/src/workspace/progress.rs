use crate::{
    Chapter, ChapterKind, ChapterProgress, CheckKind, CheckResult, CheckStatus, CompletionState,
    DomainError, ErrorCode, LoadedCourse,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Progress {
    pub(super) chapters: BTreeMap<String, ChapterProgress>,
}

#[derive(Clone)]
pub(crate) struct ProgressCourse {
    exercises: BTreeSet<String>,
}

impl ProgressCourse {
    pub(crate) fn from_course(course: &LoadedCourse) -> Self {
        Self {
            exercises: course
                .chapters
                .iter()
                .filter(|chapter| chapter.kind == ChapterKind::Exercise)
                .map(|chapter| chapter.chapter_id.clone())
                .collect(),
        }
    }
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
    pub fn completed_for(&self, course: &ProgressCourse) -> u32 {
        course
            .exercises
            .iter()
            .filter(|id| {
                self.chapters
                    .get(*id)
                    .is_some_and(|progress| progress.completion_state == CompletionState::Completed)
            })
            .count() as u32
    }

    pub(crate) fn start_checks<'a>(
        &mut self,
        chapter: &Chapter,
        check_ids: impl IntoIterator<Item = &'a str>,
    ) {
        let progress = self.chapters.entry(chapter.chapter_id.clone()).or_default();
        progress.last_check_results = check_ids.into_iter().map(CheckResult::running).collect();
        self.update_completion(chapter, false);
    }

    pub(crate) fn record_check_result(
        &mut self,
        chapter_id: &str,
        check_id: &str,
        result: CheckResult,
    ) {
        let progress = self.chapters.entry(chapter_id.to_owned()).or_default();
        if let Some(previous) = progress
            .last_check_results
            .iter_mut()
            .find(|previous| previous.check_id == check_id)
        {
            *previous = result;
        } else {
            progress.last_check_results.push(result);
        }
    }

    pub(crate) fn finish_checks(&mut self, chapter: &Chapter) {
        self.update_completion(chapter, false);
    }

    pub(crate) fn set_self_check(
        &mut self,
        chapter: &Chapter,
        check_id: &str,
        checked: bool,
    ) -> Result<(), DomainError> {
        let check = chapter
            .three_step
            .as_ref()
            .and_then(|step| {
                step.step3_checks
                    .iter()
                    .find(|check| check.id == check_id && check.kind == CheckKind::SelfCheck)
            })
            .ok_or_else(|| DomainError::new(ErrorCode::CheckNotFound))?;
        self.chapters
            .entry(chapter.chapter_id.clone())
            .or_default()
            .self_checks
            .insert(check.id.clone(), checked);
        self.update_completion(chapter, false);
        Ok(())
    }

    pub(crate) fn complete_exercise(&mut self, chapter: &Chapter) -> Result<(), DomainError> {
        if chapter.three_step.as_ref().is_some_and(|step| {
            step.step3_checks
                .iter()
                .any(|check| check.kind != CheckKind::SelfCheck)
        }) {
            return Err(DomainError::new(ErrorCode::ExerciseCompletionBlocked));
        }
        self.update_completion(chapter, true);
        if self.chapters[&chapter.chapter_id].completion_state != CompletionState::Completed {
            return Err(DomainError::new(ErrorCode::ExerciseCompletionBlocked));
        }
        Ok(())
    }

    pub(crate) fn recover_interrupted(&mut self) -> bool {
        let mut interrupted = false;
        for chapter in self.chapters.values_mut() {
            for result in &mut chapter.last_check_results {
                if result.status == CheckStatus::Running {
                    result.status = CheckStatus::Failed;
                    result.error = Some(
                        DomainError::new(ErrorCode::ExecutionFailed)
                            .with_detail("reasonCode", "VERIFICATION_INTERRUPTED"),
                    );
                    interrupted = true;
                }
            }
        }
        interrupted
    }

    pub(crate) fn has_interrupted_checks(&self) -> bool {
        self.chapters.values().any(|chapter| {
            chapter
                .last_check_results
                .iter()
                .any(|result| result.status == CheckStatus::Running)
        })
    }

    pub(crate) fn reset(&mut self) {
        self.chapters.clear();
    }

    fn update_completion(&mut self, chapter: &Chapter, manual: bool) {
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
