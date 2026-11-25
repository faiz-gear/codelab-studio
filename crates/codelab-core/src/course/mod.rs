pub(crate) mod authoring;
mod builtin;
mod builtin_solutions;
mod chapter_id;
pub(crate) mod git_pack;
mod model;
mod pack;
mod parser;
mod sanitize;
mod shell;
mod watcher;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub(crate) use builtin::builtin_solution;
pub use git_pack::GitFetchAdapter;
pub use model::{
    Chapter, ChapterKind, ChapterPresentation, CheckKind, CommandBlock, CommandMode, ContentBlock,
    CourseCheckSpec, CourseSource, CourseSourceChangedEvent, CourseSummary, DaemonRequirement,
    HttpExpectation, LoadedCourse, MAX_MARKDOWN_BYTES, OpenCourseRequest, ThreeStepProjection,
    ValueOrigin,
};
pub(crate) use watcher::CourseWatcher;

use crate::error::{DomainError, ErrorCode};

pub(crate) type ChapterReferences = HashMap<String, Vec<(String, String)>>;

pub(crate) struct OpenedCourse {
    pub course: LoadedCourse,
    pub watched_files: Vec<PathBuf>,
    pub starter: Option<Starter>,
    pub references: ChapterReferences,
}

pub(crate) type EmbeddedFiles = &'static [(&'static str, &'static [u8])];

#[derive(Clone)]
pub(crate) enum Starter {
    Builtin(EmbeddedFiles),
    Local { root: PathBuf, directory: String },
}

pub(crate) fn list_builtin_courses() -> Result<Vec<CourseSummary>, DomainError> {
    builtin::list_builtin_courses()
}

pub(crate) fn open_local_course(
    request: &OpenCourseRequest,
    data_dir: &Path,
) -> Result<OpenedCourse, DomainError> {
    match request {
        OpenCourseRequest::Builtin { course_id } => builtin::load_builtin_course(course_id),
        OpenCourseRequest::LocalMarkdown { path } => {
            protect_git_cache(data_dir, path)?;
            pack::load_local_markdown_file(path)
        }
        OpenCourseRequest::LocalPack { path } => {
            protect_git_cache(data_dir, path)?;
            pack::load_local_course_pack(path)
        }
        OpenCourseRequest::GitPack { .. } => unreachable!("Git courses are prepared separately"),
    }
}

fn protect_git_cache(data_dir: &Path, path: &str) -> Result<(), DomainError> {
    if std::fs::canonicalize(path.trim())
        .is_ok_and(|root| root.starts_with(data_dir.join(git_pack::CACHE_DIRECTORY)))
    {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid).with_detail("path", path));
    }
    Ok(())
}

pub(crate) use shell::execution_script;
