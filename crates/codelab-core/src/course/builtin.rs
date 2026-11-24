use crate::course::model::{CourseSource, CourseSummary, LoadedCourse};
use crate::course::pack::{build_course_from_pack_parts, parse_manifest_bytes};
use crate::error::{DomainError, ErrorCode};

const GO_GOFRAME_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/codelab.json");
const GO_GOFRAME_README: &str =
    include_str!("../../../../builtin-courses/go-goframe-practice/README.md");
const GO_GOFRAME_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/.codelab-checks.json");

struct BuiltinPackAsset {
    course_id: &'static str,
    manifest: &'static [u8],
    readme: &'static str,
    checks: &'static [u8],
}

const BUILTIN_PACKS: &[BuiltinPackAsset] = &[BuiltinPackAsset {
    course_id: "go-goframe-practice",
    manifest: GO_GOFRAME_MANIFEST,
    readme: GO_GOFRAME_README,
    checks: GO_GOFRAME_CHECKS,
}];

pub(crate) fn list_builtin_courses() -> Result<Vec<CourseSummary>, DomainError> {
    let mut summaries = Vec::with_capacity(BUILTIN_PACKS.len());
    for asset in BUILTIN_PACKS {
        let loaded = load_builtin_asset(asset)?;
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

pub(crate) fn load_builtin_course(course_id: &str) -> Result<LoadedCourse, DomainError> {
    let trimmed = course_id.trim();
    let asset = BUILTIN_PACKS
        .iter()
        .find(|pack| pack.course_id == trimmed)
        .ok_or_else(|| {
            DomainError::new(ErrorCode::CourseNotFound).with_detail("source", trimmed.to_string())
        })?;
    load_builtin_asset(asset)
}

fn load_builtin_asset(asset: &BuiltinPackAsset) -> Result<LoadedCourse, DomainError> {
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
