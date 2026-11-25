use crate::course::model::{CourseSource, CourseSummary, LoadedCourse};
use crate::course::pack::{build_course_from_pack_parts, parse_manifest_bytes};
use crate::course::{
    EmbeddedFiles, OpenedCourse, Starter, builtin_solutions::GO_GOFRAME_SOLUTIONS,
};
use crate::error::{DomainError, ErrorCode};

const GO_GOFRAME_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/codelab.json");
const GO_GOFRAME_README: &str =
    include_str!("../../../../builtin-courses/go-goframe-practice/README.md");
const GO_GOFRAME_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/.codelab-checks.json");

macro_rules! go_starter_file {
    ($path:literal) => {
        (
            $path,
            include_bytes!(concat!(
                "../../../../builtin-courses/go-goframe-practice/starter/",
                $path
            )),
        )
    };
}

const GO_GOFRAME_STARTER: &[(&str, &[u8])] = &[
    go_starter_file!(".gitignore"),
    go_starter_file!("REQUIREMENTS.md"),
    go_starter_file!("task-api-gf/go.mod"),
    go_starter_file!("task-api-gf/go.sum"),
    go_starter_file!("task-api-gf/main.go"),
    go_starter_file!("task-api-gf/main_test.go"),
    go_starter_file!("task-api-gf/manifest/config/config.yaml"),
    go_starter_file!("task-api/go.mod"),
    go_starter_file!("task-api/main.go"),
    go_starter_file!("task-api/main_test.go"),
    go_starter_file!("task-api/migrations/001_create_tasks.sql"),
    go_starter_file!("task-api/migrations/002_create_task_events.sql"),
];

struct BuiltinPackAsset {
    course_id: &'static str,
    manifest: &'static [u8],
    readme: &'static str,
    checks: &'static [u8],
    starter: EmbeddedFiles,
    solutions: &'static [(&'static str, EmbeddedFiles)],
}

const BUILTIN_PACKS: &[BuiltinPackAsset] = &[BuiltinPackAsset {
    course_id: "go-goframe-practice",
    manifest: GO_GOFRAME_MANIFEST,
    readme: GO_GOFRAME_README,
    checks: GO_GOFRAME_CHECKS,
    starter: GO_GOFRAME_STARTER,
    solutions: GO_GOFRAME_SOLUTIONS,
}];

pub(crate) fn builtin_solution(course_id: &str, chapter_id: &str) -> Option<EmbeddedFiles> {
    BUILTIN_PACKS
        .iter()
        .find(|pack| pack.course_id == course_id)?
        .solutions
        .iter()
        .find(|(id, _)| *id == chapter_id)
        .map(|(_, files)| *files)
}

pub(crate) fn list_builtin_courses() -> Result<Vec<CourseSummary>, DomainError> {
    let mut summaries = Vec::with_capacity(BUILTIN_PACKS.len());
    for asset in BUILTIN_PACKS {
        let (loaded, _) = load_builtin_asset(asset)?;
        summaries.push(CourseSummary {
            course_id: loaded.course_id,
            title: loaded.title,
            description: loaded.description,
            language: loaded.language,
            source: loaded.source,
            total_chapters: loaded.chapters.len() as u32,
            total_exercises: loaded.total_exercises,
            is_builtin: true,
        });
    }
    Ok(summaries)
}

pub(crate) fn load_builtin_course(course_id: &str) -> Result<OpenedCourse, DomainError> {
    let trimmed = course_id.trim();
    let asset = BUILTIN_PACKS
        .iter()
        .find(|pack| pack.course_id == trimmed)
        .ok_or_else(|| {
            DomainError::new(ErrorCode::CourseNotFound).with_detail("source", trimmed.to_string())
        })?;
    let (course, references) = load_builtin_asset(asset)?;
    Ok(OpenedCourse {
        course,
        references,
        watched_files: Vec::new(),
        starter: Some(Starter::Builtin(asset.starter)),
    })
}

fn load_builtin_asset(
    asset: &BuiltinPackAsset,
) -> Result<(LoadedCourse, super::ChapterReferences), DomainError> {
    let manifest = parse_manifest_bytes(asset.manifest)?;
    build_course_from_pack_parts(
        manifest,
        asset.readme,
        Some(asset.checks),
        CourseSource::Builtin {
            course_id: asset.course_id.to_string(),
        },
        None,
        true,
    )
}
