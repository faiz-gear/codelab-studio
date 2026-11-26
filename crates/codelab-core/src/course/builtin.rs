use crate::course::model::{CourseSource, CourseSummary, LoadedCourse};
use crate::course::pack::{build_course_from_pack_parts, parse_manifest_bytes};
use crate::course::{
    EmbeddedFiles, OpenedCourse, Starter,
    builtin_solutions::{
        GO_GOFRAME_SOLUTIONS, JAVA_RATE_LIMITER_SOLUTIONS, PYTHON_DATA_CLEAN_SOLUTIONS,
        TS_DATA_PIPELINE_SOLUTIONS,
    },
};
use crate::error::{DomainError, ErrorCode};

macro_rules! starter_file {
    ($pack:literal, $path:literal) => {
        (
            $path,
            include_bytes!(concat!(
                "../../../../builtin-courses/",
                $pack,
                "/starter/",
                $path
            )),
        )
    };
}

const GO_GOFRAME_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/codelab.json");
const GO_GOFRAME_README: &str =
    include_str!("../../../../builtin-courses/go-goframe-practice/README.md");
const GO_GOFRAME_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/go-goframe-practice/.codelab-checks.json");

const GO_GOFRAME_STARTER: &[(&str, &[u8])] = &[
    starter_file!("go-goframe-practice", ".gitignore"),
    starter_file!("go-goframe-practice", "REQUIREMENTS.md"),
    starter_file!("go-goframe-practice", "task-api-gf/go.mod"),
    starter_file!("go-goframe-practice", "task-api-gf/go.sum"),
    starter_file!("go-goframe-practice", "task-api-gf/main.go"),
    starter_file!("go-goframe-practice", "task-api-gf/main_test.go"),
    starter_file!(
        "go-goframe-practice",
        "task-api-gf/manifest/config/config.yaml"
    ),
    starter_file!("go-goframe-practice", "task-api/go.mod"),
    starter_file!("go-goframe-practice", "task-api/main.go"),
    starter_file!("go-goframe-practice", "task-api/main_test.go"),
    starter_file!(
        "go-goframe-practice",
        "task-api/migrations/001_create_tasks.sql"
    ),
    starter_file!(
        "go-goframe-practice",
        "task-api/migrations/002_create_task_events.sql"
    ),
];

const TS_DATA_PIPELINE_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/ts-data-pipeline/codelab.json");
const TS_DATA_PIPELINE_README: &str =
    include_str!("../../../../builtin-courses/ts-data-pipeline/README.md");
const TS_DATA_PIPELINE_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/ts-data-pipeline/.codelab-checks.json");

const TS_DATA_PIPELINE_STARTER: &[(&str, &[u8])] = &[
    starter_file!("ts-data-pipeline", "package.json"),
    starter_file!("ts-data-pipeline", "tsconfig.json"),
    starter_file!("ts-data-pipeline", "src/types.ts"),
    starter_file!("ts-data-pipeline", "src/filter.ts"),
    starter_file!("ts-data-pipeline", "src/normalize.ts"),
    starter_file!("ts-data-pipeline", "src/dedup.ts"),
    starter_file!("ts-data-pipeline", "src/aggregate.ts"),
    starter_file!("ts-data-pipeline", "src/paginate.ts"),
    starter_file!("ts-data-pipeline", "src/settled.ts"),
    starter_file!("ts-data-pipeline", "src/concurrency.ts"),
    starter_file!("ts-data-pipeline", "src/pipeline.ts"),
    starter_file!("ts-data-pipeline", "tests/ex1-filter.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex2-normalize.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex3-dedup.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex4-aggregate.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex5-paginate.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex6-settled.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex7-concurrency.test.ts"),
    starter_file!("ts-data-pipeline", "tests/ex8-pipeline.test.ts"),
];

const PYTHON_DATA_CLEAN_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/python-data-clean/codelab.json");
const PYTHON_DATA_CLEAN_README: &str =
    include_str!("../../../../builtin-courses/python-data-clean/README.md");
const PYTHON_DATA_CLEAN_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/python-data-clean/.codelab-checks.json");

const PYTHON_DATA_CLEAN_STARTER: &[(&str, &[u8])] = &[
    starter_file!("python-data-clean", "pyproject.toml"),
    starter_file!("python-data-clean", "src/models.py"),
    starter_file!("python-data-clean", "src/cleaner.py"),
    starter_file!("python-data-clean", "src/csv_loader.py"),
    starter_file!("python-data-clean", "src/dedup.py"),
    starter_file!("python-data-clean", "src/analytics.py"),
    starter_file!("python-data-clean", "src/anomaly.py"),
    starter_file!("python-data-clean", "src/report.py"),
    starter_file!("python-data-clean", "tests/conftest.py"),
    starter_file!("python-data-clean", "tests/test_ex1_cleaner.py"),
    starter_file!("python-data-clean", "tests/test_ex2_csv_loader.py"),
    starter_file!("python-data-clean", "tests/test_ex3_dedup.py"),
    starter_file!("python-data-clean", "tests/test_ex4_analytics.py"),
    starter_file!("python-data-clean", "tests/test_ex5_anomaly.py"),
    starter_file!("python-data-clean", "tests/test_ex6_report.py"),
];

const JAVA_RATE_LIMITER_MANIFEST: &[u8] =
    include_bytes!("../../../../builtin-courses/java-rate-limiter/codelab.json");
const JAVA_RATE_LIMITER_README: &str =
    include_str!("../../../../builtin-courses/java-rate-limiter/README.md");
const JAVA_RATE_LIMITER_CHECKS: &[u8] =
    include_bytes!("../../../../builtin-courses/java-rate-limiter/.codelab-checks.json");

const JAVA_RATE_LIMITER_STARTER: &[(&str, &[u8])] = &[
    starter_file!("java-rate-limiter", "pom.xml"),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/Clock.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/RateLimiter.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/RateLimitResult.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/TokenBucket.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/RefillingTokenBucket.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/FixedWindowLimiter.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/SlidingWindowLogLimiter.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/TenantRateLimiter.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/VirtualThreadLoadRunner.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/main/java/com/codelab/ratelimiter/CompositeRateLimiter.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex1TokenBucketTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex2RefillingTokenBucketTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex3FixedWindowLimiterTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex4SlidingWindowLogLimiterTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex5TenantRateLimiterTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex6VirtualThreadLoadRunnerTest.java"
    ),
    starter_file!(
        "java-rate-limiter",
        "src/test/java/com/codelab/ratelimiter/Ex7CompositeRateLimiterTest.java"
    ),
];

struct BuiltinPackAsset {
    course_id: &'static str,
    manifest: &'static [u8],
    readme: &'static str,
    checks: &'static [u8],
    starter: EmbeddedFiles,
    solutions: &'static [(&'static str, EmbeddedFiles)],
}

const BUILTIN_PACKS: &[BuiltinPackAsset] = &[
    BuiltinPackAsset {
        course_id: "go-goframe-practice",
        manifest: GO_GOFRAME_MANIFEST,
        readme: GO_GOFRAME_README,
        checks: GO_GOFRAME_CHECKS,
        starter: GO_GOFRAME_STARTER,
        solutions: GO_GOFRAME_SOLUTIONS,
    },
    BuiltinPackAsset {
        course_id: "ts-data-pipeline",
        manifest: TS_DATA_PIPELINE_MANIFEST,
        readme: TS_DATA_PIPELINE_README,
        checks: TS_DATA_PIPELINE_CHECKS,
        starter: TS_DATA_PIPELINE_STARTER,
        solutions: TS_DATA_PIPELINE_SOLUTIONS,
    },
    BuiltinPackAsset {
        course_id: "python-data-clean",
        manifest: PYTHON_DATA_CLEAN_MANIFEST,
        readme: PYTHON_DATA_CLEAN_README,
        checks: PYTHON_DATA_CLEAN_CHECKS,
        starter: PYTHON_DATA_CLEAN_STARTER,
        solutions: PYTHON_DATA_CLEAN_SOLUTIONS,
    },
    BuiltinPackAsset {
        course_id: "java-rate-limiter",
        manifest: JAVA_RATE_LIMITER_MANIFEST,
        readme: JAVA_RATE_LIMITER_README,
        checks: JAVA_RATE_LIMITER_CHECKS,
        starter: JAVA_RATE_LIMITER_STARTER,
        solutions: JAVA_RATE_LIMITER_SOLUTIONS,
    },
];

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
