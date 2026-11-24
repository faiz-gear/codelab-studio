mod builtin;
mod chapter_id;
mod model;
mod pack;
mod parser;
mod sanitize;
mod shell;
mod watcher;

use std::path::PathBuf;

pub use model::{
    Chapter, ChapterKind, ChapterPresentation, CheckKind, CommandBlock, CommandMode, ContentBlock,
    CourseCheckSpec, CourseSource, CourseSourceChangedEvent, CourseSummary, DaemonRequirement,
    LoadedCourse, MAX_MARKDOWN_BYTES, OpenCourseRequest, ThreeStepProjection, ValueOrigin,
};
pub(crate) use watcher::CourseWatcher;

use crate::error::DomainError;

pub(crate) struct OpenedCourse {
    pub course: LoadedCourse,
    pub watched_files: Vec<PathBuf>,
}

pub(crate) fn list_builtin_courses() -> Result<Vec<CourseSummary>, DomainError> {
    builtin::list_builtin_courses()
}

pub(crate) fn open_course(request: &OpenCourseRequest) -> Result<OpenedCourse, DomainError> {
    match request {
        OpenCourseRequest::Builtin { course_id } => {
            let course = builtin::load_builtin_course(course_id)?;
            Ok(OpenedCourse {
                course,
                watched_files: Vec::new(),
            })
        }
        OpenCourseRequest::LocalMarkdown { path } => pack::load_local_markdown_file(path),
        OpenCourseRequest::LocalPack { path } => pack::load_local_course_pack(path),
    }
}
