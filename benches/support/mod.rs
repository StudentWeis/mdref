use std::{
    fs,
    hint::black_box,
    io,
    path::{Path, PathBuf},
};

use mdref::{Result, mv};
use pathdiff::diff_paths;
use tempfile::TempDir;

const FIXED_MARKDOWN_FILES: usize = 4;
const LINKS_PER_CONTENT_DOCUMENT: usize = 6;
const BUNDLE_HOT_FILE_REFERENCES: usize = 3;
const HOT_FILE_BUNDLE_REFERENCES: usize = 2;
const BUNDLE_INTERNAL_REFERENCES: usize = 3;
const BUNDLE_OUTBOUND_REFERENCES: usize = 3;
const PAYLOAD_LINE: &str =
    "Benchmark payload text to keep parsing work realistic without making fixtures enormous.\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureProfile {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSummary {
    pub content_directories: usize,
    pub content_documents: usize,
    pub markdown_files: usize,
    pub total_markdown_bytes: usize,
    pub representative_document_bytes: usize,
    pub links_per_content_document: usize,
    pub hot_file_references: usize,
    pub bundle_directory_references: usize,
    pub directory_move_rewrites: usize,
}

#[derive(Debug)]
pub struct BenchmarkFixture {
    _temp_dir: TempDir,
    pub root: PathBuf,
    pub hot_file: PathBuf,
    pub hot_directory: PathBuf,
    pub representative_document: PathBuf,
    pub move_file_destination: PathBuf,
    pub move_directory_destination: PathBuf,
    pub summary: FixtureSummary,
}

#[derive(Debug, Clone, Copy)]
pub struct MoveOperation<'a> {
    pub source: &'a Path,
    pub destination: &'a Path,
    pub root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
struct FixtureConfig {
    label: &'static str,
    levels: usize,
    branches_per_level: usize,
    documents_per_directory: usize,
    payload_repetitions: usize,
}

impl FixtureProfile {
    pub const fn label(self) -> &'static str {
        self.config().label
    }

    const fn config(self) -> FixtureConfig {
        match self {
            Self::Small => FixtureConfig {
                label: "small",
                levels: 2,
                branches_per_level: 2,
                documents_per_directory: 4,
                payload_repetitions: 4,
            },
            Self::Medium => FixtureConfig {
                label: "medium",
                levels: 3,
                branches_per_level: 2,
                documents_per_directory: 6,
                payload_repetitions: 8,
            },
            Self::Large => FixtureConfig {
                label: "large",
                levels: 4,
                branches_per_level: 3,
                documents_per_directory: 4,
                payload_repetitions: 12,
            },
        }
    }
}

impl BenchmarkFixture {
    pub fn file_move_operation(&self) -> MoveOperation<'_> {
        MoveOperation {
            source: &self.hot_file,
            destination: &self.move_file_destination,
            root: &self.root,
        }
    }

    pub fn directory_move_operation(&self) -> MoveOperation<'_> {
        MoveOperation {
            source: &self.hot_directory,
            destination: &self.move_directory_destination,
            root: &self.root,
        }
    }
}

pub fn run_move_operation(operation: MoveOperation<'_>) -> Result<()> {
    mv(
        black_box(operation.source),
        black_box(operation.destination),
        black_box(operation.root),
        false,
    )
}

pub fn build_fixture(profile: FixtureProfile) -> io::Result<BenchmarkFixture> {
    let config = profile.config();
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().to_path_buf();

    let assets_dir = root.join("assets");
    let targets_dir = root.join("targets");
    let bundle_dir = root.join("bundle");
    let bundle_nested_dir = bundle_dir.join("nested");
    let content_root = root.join("content");

    fs::create_dir_all(&assets_dir)?;
    fs::create_dir_all(&targets_dir)?;
    fs::create_dir_all(&bundle_nested_dir)?;
    fs::create_dir_all(&content_root)?;

    fs::write(
        assets_dir.join("diagram.png"),
        b"not-a-real-png-but-good-enough-for-benchmarks",
    )?;

    let hot_file = targets_dir.join("hot.md");
    let bundle_index = bundle_dir.join("index.md");
    let bundle_guide = bundle_dir.join("guide.md");
    let bundle_checklist = bundle_nested_dir.join("checklist.md");

    let mut total_markdown_bytes = 0usize;

    total_markdown_bytes += write_markdown_file(
        &bundle_index,
        &fixed_bundle_index_content(&bundle_index, &bundle_guide, &hot_file, config),
    )?;
    total_markdown_bytes += write_markdown_file(
        &bundle_guide,
        &fixed_bundle_guide_content(&bundle_guide, &bundle_checklist, &hot_file, config),
    )?;
    total_markdown_bytes += write_markdown_file(
        &bundle_checklist,
        &fixed_bundle_checklist_content(&bundle_checklist, &bundle_guide, &hot_file, config),
    )?;
    total_markdown_bytes += write_markdown_file(
        &hot_file,
        &fixed_hot_file_content(&hot_file, &bundle_guide, &bundle_index, &assets_dir, config),
    )?;

    let content_directories =
        collect_content_directories(&content_root, config.levels, config.branches_per_level);

    let mut representative_document = None;
    let mut representative_document_bytes = 0usize;
    let mut content_documents = 0usize;

    for directory in &content_directories {
        fs::create_dir_all(directory)?;
        for document_index in 0..config.documents_per_directory {
            let document = directory.join(format!("doc_{document_index}.md"));
            let content = content_document_content(
                &document,
                &hot_file,
                &bundle_guide,
                &bundle_index,
                &assets_dir.join("diagram.png"),
                config,
            );
            let bytes = write_markdown_file(&document, &content)?;
            if representative_document.is_none() {
                representative_document = Some(document.clone());
                representative_document_bytes = bytes;
            }
            total_markdown_bytes += bytes;
            content_documents += 1;
        }
    }

    let content_directories_len = content_directories.len();
    let summary = FixtureSummary {
        content_directories: content_directories_len,
        content_documents,
        markdown_files: content_documents + FIXED_MARKDOWN_FILES,
        total_markdown_bytes,
        representative_document_bytes,
        links_per_content_document: LINKS_PER_CONTENT_DOCUMENT,
        hot_file_references: (content_documents * 2) + BUNDLE_HOT_FILE_REFERENCES,
        bundle_directory_references: (content_documents * 2)
            + HOT_FILE_BUNDLE_REFERENCES
            + BUNDLE_INTERNAL_REFERENCES,
        directory_move_rewrites: (content_documents * 2)
            + HOT_FILE_BUNDLE_REFERENCES
            + BUNDLE_INTERNAL_REFERENCES
            + BUNDLE_OUTBOUND_REFERENCES,
    };

    Ok(BenchmarkFixture {
        _temp_dir: temp_dir,
        root: root.clone(),
        hot_file,
        hot_directory: bundle_dir,
        representative_document: representative_document
            .expect("fixture has at least one document"),
        move_file_destination: root.join("archive").join("hot.md"),
        move_directory_destination: root.join("archive").join("bundle"),
        summary,
    })
}

fn collect_content_directories(base: &Path, levels: usize, branches: usize) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    collect_content_directories_inner(base, levels, branches, &mut directories);
    directories
}

fn collect_content_directories_inner(
    current: &Path,
    levels: usize,
    branches: usize,
    directories: &mut Vec<PathBuf>,
) {
    directories.push(current.to_path_buf());

    if levels <= 1 {
        return;
    }

    for branch in 0..branches {
        let child = current.join(format!("section_{branch}"));
        collect_content_directories_inner(&child, levels - 1, branches, directories);
    }
}

fn fixed_hot_file_content(
    current_file: &Path,
    bundle_guide: &Path,
    bundle_index: &Path,
    assets_dir: &Path,
    config: FixtureConfig,
) -> String {
    let diagram = assets_dir.join("diagram.png");
    let guide_link = relative_link(current_file, bundle_guide);
    let index_link = relative_link(current_file, bundle_index);
    let diagram_link = relative_link(current_file, &diagram);

    format!(
        "# Hot Target\n\n[Bundle guide]({guide_link})\n[Bundle index][bundle-index]\n![Diagram]({diagram_link})\n\n[bundle-index]: {index_link}\n\n{}",
        payload(config.payload_repetitions),
    )
}

fn fixed_bundle_index_content(
    current_file: &Path,
    bundle_guide: &Path,
    hot_file: &Path,
    config: FixtureConfig,
) -> String {
    let guide_link = relative_link(current_file, bundle_guide);
    let hot_link = relative_link(current_file, hot_file);

    format!(
        "# Bundle Index\n\n[Guide]({guide_link})\n[Hot target]({hot_link})\n\n{}",
        payload(config.payload_repetitions),
    )
}

fn fixed_bundle_guide_content(
    current_file: &Path,
    bundle_checklist: &Path,
    hot_file: &Path,
    config: FixtureConfig,
) -> String {
    let checklist_link = relative_link(current_file, bundle_checklist);
    let hot_link = relative_link(current_file, hot_file);

    format!(
        "# Bundle Guide\n\n[Checklist]({checklist_link})\n[Hot target]({hot_link})\n\n{}",
        payload(config.payload_repetitions),
    )
}

fn fixed_bundle_checklist_content(
    current_file: &Path,
    bundle_guide: &Path,
    hot_file: &Path,
    config: FixtureConfig,
) -> String {
    let guide_link = relative_link(current_file, bundle_guide);
    let hot_link = relative_link(current_file, hot_file);

    format!(
        "# Bundle Checklist\n\n[Guide]({guide_link})\n[Hot target]({hot_link})\n\n{}",
        payload(config.payload_repetitions),
    )
}

fn content_document_content(
    current_file: &Path,
    hot_file: &Path,
    bundle_guide: &Path,
    bundle_index: &Path,
    diagram: &Path,
    config: FixtureConfig,
) -> String {
    let hot_link = relative_link(current_file, hot_file);
    let guide_link = relative_link(current_file, bundle_guide);
    let index_link = relative_link(current_file, bundle_index);
    let diagram_link = relative_link(current_file, diagram);

    format!(
        "# Benchmark Document\n\n[Hot target]({hot_link})\n[Bundle guide]({guide_link})\n[Local notes](./notes.md)\n![Diagram]({diagram_link})\n\n[Hot reference][hot-ref]\n[Bundle index][bundle-index]\n\n[hot-ref]: {hot_link}\n[bundle-index]: {index_link}\n\n{}",
        payload(config.payload_repetitions),
    )
}

fn payload(repetitions: usize) -> String {
    PAYLOAD_LINE.repeat(repetitions)
}

fn relative_link(from_file: &Path, to: &Path) -> String {
    let parent = from_file
        .parent()
        .expect("benchmark fixture files always have a parent directory");
    let relative_path = diff_paths(to, parent).expect("paths share the same tempdir root");
    markdown_link_path(&relative_path)
}

fn markdown_link_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn write_markdown_file(path: &Path, content: &str) -> io::Result<usize> {
    fs::write(path, content)?;
    Ok(content.len())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_markdown_link_path_normalizes_backslashes() {
        assert_eq!(
            super::markdown_link_path(std::path::Path::new("..\\bundle\\index.md")),
            "../bundle/index.md"
        );
    }

    #[test]
    fn test_markdown_link_path_preserves_forward_slashes() {
        assert_eq!(
            super::markdown_link_path(std::path::Path::new("../bundle/index.md")),
            "../bundle/index.md"
        );
    }
}
