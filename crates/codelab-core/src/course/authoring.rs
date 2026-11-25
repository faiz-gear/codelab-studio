use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::chapter_id::unicode_nfkc_slug;
use super::{LoadedCourse, OpenedCourse, pack};
use crate::filesystem::{open_canonical_dir, open_dir_at};
use crate::workspace::WorkspacePaths;
use crate::{CourseLanguage, DomainError, ErrorCode, WorkspaceEntryKind, WorkspaceFileContents};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDraftRequest {
    pub path: String,
    pub title: String,
    pub language: CourseLanguage,
    pub reference_solution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDraft {
    pub id: u64,
    pub root: String,
    pub files: Vec<String>,
    pub preview: DraftPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftFilePath {
    pub draft_id: u64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftFileWrite {
    #[serde(flatten)]
    pub target: DraftFilePath,
    pub text: String,
    pub expected_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftEdit {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPreviewRequest {
    pub draft_id: u64,
    pub edits: Vec<DraftEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPreview {
    pub course: Option<LoadedCourse>,
    pub diagnostics: Vec<DraftDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDiagnostic {
    pub path: String,
    pub error: DomainError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialWorkspaceOpened {
    pub course: LoadedCourse,
    pub workspace: crate::WorkspaceSummary,
}

pub(crate) struct Draft {
    pub id: u64,
    pub paths: WorkspacePaths,
}

impl Draft {
    pub fn create(
        id: u64,
        request: CourseDraftRequest,
        data_dir: &Path,
        workspace_root: Option<&Path>,
    ) -> Result<Self, DomainError> {
        let title = request.title.trim();
        if title.is_empty() || title.chars().any(char::is_control) {
            return Err(DomainError::new(ErrorCode::InvalidCourseMetadata));
        }
        let path = checked_input(&request.path)?;
        let root = match fs::canonicalize(path) {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let parent = fs::canonicalize(path.parent().ok_or_else(invalid_draft_path)?)
                    .map_err(|_| invalid_draft_path())?;
                let name = path.file_name().ok_or_else(invalid_draft_path)?;
                let root = parent.join(name);
                check_location(&root, data_dir, workspace_root)?;
                open_canonical_dir(&parent)
                    .and_then(|dir| dir.create_dir(name))
                    .map_err(|_| invalid_draft_path())?;
                root
            }
            Err(_) => return Err(invalid_draft_path()),
        };
        check_location(&root, data_dir, workspace_root)?;
        let paths = WorkspacePaths::open(root, None).map_err(|_| invalid_draft_path())?;
        if paths
            .dir
            .entries()
            .map_err(|_| invalid_draft_path())?
            .next()
            .is_some()
        {
            return Err(DomainError::new(ErrorCode::DraftDirectoryNotEmpty));
        }
        scaffold(
            &paths.dir,
            title,
            request.language,
            request.reference_solution,
        )
        .map_err(|_| DomainError::new(ErrorCode::DraftWriteFailed))?;
        Ok(Self { id, paths })
    }

    pub fn open(
        id: u64,
        path: &str,
        data_dir: &Path,
        workspace_root: Option<&Path>,
    ) -> Result<Self, DomainError> {
        let root = fs::canonicalize(checked_input(path)?).map_err(|_| invalid_draft_path())?;
        check_location(&root, data_dir, workspace_root)?;
        let paths = WorkspacePaths::open(root, None).map_err(|_| invalid_draft_path())?;
        if !paths
            .dir
            .symlink_metadata("codelab.json")
            .is_ok_and(|file| file.is_file())
            || !paths.dir.try_exists(".codelab").is_ok_and(|exists| !exists)
        {
            return Err(invalid_draft_path());
        }
        Ok(Self { id, paths })
    }

    pub fn summary(&self) -> Result<CourseDraft, DomainError> {
        let mut files = Vec::new();
        let mut directories = vec![String::new()];
        while let Some(directory) = directories.pop() {
            for entry in self.paths.list(&directory)? {
                match entry.kind {
                    WorkspaceEntryKind::Directory => directories.push(entry.path),
                    WorkspaceEntryKind::File => files.push(entry.path),
                }
            }
        }
        files.sort();
        Ok(CourseDraft {
            id: self.id,
            root: self.paths.root.to_string_lossy().into_owned(),
            files,
            preview: self.preview(&[])?,
        })
    }

    pub fn preview(&self, edits: &[DraftEdit]) -> Result<DraftPreview, DomainError> {
        // Check the draft root identity and resolve edits as existing text files.
        self.paths.input("").map_err(|_| invalid_draft_path())?;
        let mut overrides = HashMap::new();
        for edit in edits {
            let file = self.paths.read(&edit.path)?;
            if !matches!(file.contents, WorkspaceFileContents::Text { .. })
                || edit.text.len() > crate::MAX_WORKSPACE_TEXT_BYTES
            {
                return Err(DomainError::new(ErrorCode::WorkspaceFileNotText)
                    .with_detail("path", &edit.path));
            }
            overrides.insert(self.paths.root.join(file.path), edit.text.clone());
        }
        let mut errors = Vec::new();
        let course = match pack::load_pack(
            &self.paths.root.to_string_lossy(),
            &overrides,
            Some(&mut errors),
        ) {
            Ok(opened) => Some(opened.course),
            Err(error) => {
                errors.push(error);
                None
            }
        };
        let diagnostics = errors
            .into_iter()
            .map(|error| DraftDiagnostic {
                path: error
                    .details
                    .get("path")
                    .cloned()
                    .unwrap_or_else(|| "codelab.json".into()),
                error,
            })
            .collect();
        Ok(DraftPreview {
            course,
            diagnostics,
        })
    }

    pub fn trial_course(&self) -> Result<OpenedCourse, DomainError> {
        self.paths.input("").map_err(|_| invalid_draft_path())?;
        let mut opened = pack::load_local_course_pack(&self.paths.root.to_string_lossy())?;
        opened.course.trusted = true;
        opened.watched_files.clear();
        Ok(opened)
    }
}

fn checked_input(path: &str) -> Result<&Path, DomainError> {
    let path = Path::new(path.trim());
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        return Err(invalid_draft_path());
    }
    Ok(path)
}

fn check_location(root: &Path, data: &Path, workspace: Option<&Path>) -> Result<(), DomainError> {
    let overlaps = |other: &Path| root.starts_with(other) || other.starts_with(root);
    if overlaps(data) || workspace.is_some_and(overlaps) {
        return Err(invalid_draft_path());
    }
    Ok(())
}

fn invalid_draft_path() -> DomainError {
    DomainError::new(ErrorCode::InvalidDraftPath)
}

fn scaffold(
    root: &Dir,
    title: &str,
    language: CourseLanguage,
    reference_solution: bool,
) -> io::Result<()> {
    let (file, command, starter, solution) = match language {
        CourseLanguage::Go => (
            "main.go",
            "go run main.go",
            "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"Hello\")\n}\n",
            "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"Hello, CodeLab!\")\n}\n",
        ),
        CourseLanguage::JsTs => (
            "main.ts",
            "bun run main.ts",
            "console.log(\"Hello\");\n",
            "console.log(\"Hello, CodeLab!\");\n",
        ),
        CourseLanguage::Python => (
            "main.py",
            "python3 main.py",
            "print(\"Hello\")\n",
            "print(\"Hello, CodeLab!\")\n",
        ),
        CourseLanguage::Java => (
            "Main.java",
            "java Main.java",
            "class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello\");\n    }\n}\n",
            "class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, CodeLab!\");\n    }\n}\n",
        ),
    };
    let manifest = json!({
        "schemaVersion": "1.0", "courseId": unicode_nfkc_slug(title),
        "title": title, "language": language, "entryFile": "README.md",
        "checksFile": ".codelab-checks.json", "starterDir": "starter",
        "chapters": [{ "chapterId": "ex-1", "kind": "exercise", "match": { "anchor": "ex-1" }, "entryFile": file }]
    });
    let checks = json!({
        "schemaVersion": "1.0", "chapters": [{
            "chapterId": "ex-1",
            "checks": [{ "id": "greeting", "title": "The program prints Hello, CodeLab!", "type": "self_check" }]
        }]
    });
    let manual = format!(
        "# {title}\n\n## Exercise 1 {{#ex-1}}\n\nRun the example:\n\n```bash mode=oneshot\n{command}\n```\n\nUpdate `{file}` to print `Hello, CodeLab!`.\n\n### Verification\n\nRun the program again and check the greeting.\n",
    );
    let mut files = vec![
        (
            PathBuf::from("codelab.json"),
            format!("{}\n", serde_json::to_string_pretty(&manifest)?),
        ),
        (PathBuf::from("README.md"), manual),
        (
            PathBuf::from(".codelab-checks.json"),
            format!("{}\n", serde_json::to_string_pretty(&checks)?),
        ),
        (PathBuf::from("starter").join(file), starter.into()),
    ];
    let mut created_dirs = Vec::new();
    let mut created_files = Vec::new();
    let result = (|| {
        root.create_dir("starter")?;
        created_dirs.push(Path::new("starter"));
        if reference_solution {
            root.create_dir("solutions")?;
            created_dirs.push(Path::new("solutions"));
            root.open_dir_nofollow("solutions")?.create_dir("ex-1")?;
            created_dirs.push(Path::new("solutions/ex-1"));
            files.push((PathBuf::from("solutions/ex-1").join(file), solution.into()));
        }
        for (path, text) in &files {
            let parent = open_dir_at(root, path.parent().expect("scaffold file has parent"))?;
            let name = path.file_name().expect("scaffold file has name");
            let mut file =
                parent.open_with(name, OpenOptions::new().write(true).create_new(true))?;
            created_files.push(path.clone());
            file.write_all(text.as_bytes())?;
        }
        Ok(())
    })();
    if result.is_err() {
        for path in created_files.into_iter().rev() {
            let _ = root.remove_file(path);
        }
        for path in created_dirs.into_iter().rev() {
            let _ = root.remove_dir(path);
        }
    }
    result
}
