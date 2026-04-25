#[allow(dead_code)]
#[path = "../benches/support/mod.rs"]
mod support;

use std::path::Path;

use mdref::{LinkType, NoopProgress, find_links, find_references};
use rstest::rstest;
use support::{
    BenchmarkFixture, FixtureProfile, FixtureSummary, MoveOperation, build_fixture,
    run_move_operation,
};

#[test]
#[allow(clippy::unwrap_used)]
fn test_small_profile_reports_expected_summary() {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    assert_eq!(fixture.summary.content_directories, 3);
    assert_eq!(fixture.summary.content_documents, 12);
    assert_eq!(fixture.summary.markdown_files, 16);
    assert_eq!(fixture.summary.links_per_content_document, 6);
    assert_eq!(fixture.summary.hot_file_references, 27);
    assert_eq!(fixture.summary.bundle_directory_references, 29);
    assert_eq!(fixture.summary.directory_move_rewrites, 32);
    assert!(fixture.hot_file.exists());
    assert!(fixture.hot_directory.exists());
    assert!(fixture.representative_document.exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_fixture_reference_counts_match_summary() {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    let hot_file_references =
        find_references(&fixture.hot_file, &fixture.root, &NoopProgress).unwrap();
    let bundle_references =
        find_references(&fixture.hot_directory, &fixture.root, &NoopProgress).unwrap();

    assert_eq!(
        hot_file_references.len(),
        fixture.summary.hot_file_references
    );
    assert_eq!(
        bundle_references.len(),
        fixture.summary.bundle_directory_references
    );
}

#[rstest]
#[case::small(FixtureProfile::Small, 32)]
#[case::medium(FixtureProfile::Medium, 92)]
#[case::large(FixtureProfile::Large, 328)]
#[allow(clippy::unwrap_used)]
fn test_fixture_summary_directory_move_rewrites_all_profiles_match_total_rewrites(
    #[case] profile: FixtureProfile,
    #[case] expected_rewrites: usize,
) {
    let fixture = build_fixture(profile).unwrap();

    assert_eq!(fixture.summary.directory_move_rewrites, expected_rewrites);
    assert_eq!(
        fixture.summary.directory_move_rewrites,
        fixture.summary.bundle_directory_references + 3
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_representative_document_contains_expected_link_mix() {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    let links = find_links(&fixture.representative_document).unwrap();

    assert_eq!(links.len(), fixture.summary.links_per_content_document);
    assert!(
        links
            .iter()
            .any(|reference| reference.link_text.ends_with("hot.md"))
    );
    assert!(
        links
            .iter()
            .any(|reference| reference.link_text.ends_with("diagram.png"))
    );
    assert!(links.iter().any(|reference| {
        matches!(reference.link_type, LinkType::ReferenceDefinition)
            && reference.link_text.ends_with("bundle/index.md")
    }));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_move_destinations_are_reserved_but_missing() {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    assert!(!fixture.move_file_destination.exists());
    assert!(!fixture.move_directory_destination.exists());
    assert_eq!(
        fixture.move_file_destination.parent().unwrap(),
        fixture.root.join("archive")
    );
    assert_eq!(
        fixture.move_directory_destination.parent().unwrap(),
        fixture.root.join("archive")
    );
}

fn select_file_move_operation(fixture: &BenchmarkFixture) -> MoveOperation<'_> {
    fixture.file_move_operation()
}

fn select_directory_move_operation(fixture: &BenchmarkFixture) -> MoveOperation<'_> {
    fixture.directory_move_operation()
}

fn file_source_path(fixture: &BenchmarkFixture) -> &Path {
    &fixture.hot_file
}

fn file_destination_path(fixture: &BenchmarkFixture) -> &Path {
    &fixture.move_file_destination
}

fn file_reference_count(summary: &FixtureSummary) -> usize {
    summary.hot_file_references
}

fn directory_source_path(fixture: &BenchmarkFixture) -> &Path {
    &fixture.hot_directory
}

fn directory_destination_path(fixture: &BenchmarkFixture) -> &Path {
    &fixture.move_directory_destination
}

fn directory_reference_count(summary: &FixtureSummary) -> usize {
    summary.bundle_directory_references
}

#[rstest]
#[case::file(select_file_move_operation, file_source_path, file_destination_path)]
#[case::directory(
    select_directory_move_operation,
    directory_source_path,
    directory_destination_path
)]
#[allow(clippy::unwrap_used)]
fn test_move_operation_variant_selects_expected_paths(
    #[case] select_operation: for<'a> fn(&'a BenchmarkFixture) -> MoveOperation<'a>,
    #[case] select_source: fn(&BenchmarkFixture) -> &Path,
    #[case] select_destination: fn(&BenchmarkFixture) -> &Path,
) {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    let operation = select_operation(&fixture);

    assert_eq!(operation.source, select_source(&fixture));
    assert_eq!(operation.destination, select_destination(&fixture));
    assert_eq!(operation.root, fixture.root.as_path());
}

#[rstest]
#[case::file(
    select_file_move_operation,
    file_source_path,
    file_destination_path,
    file_reference_count
)]
#[case::directory(
    select_directory_move_operation,
    directory_source_path,
    directory_destination_path,
    directory_reference_count
)]
#[allow(clippy::unwrap_used)]
fn test_move_operation_execution_updates_paths_and_references(
    #[case] select_operation: for<'a> fn(&'a BenchmarkFixture) -> MoveOperation<'a>,
    #[case] select_source: fn(&BenchmarkFixture) -> &Path,
    #[case] select_destination: fn(&BenchmarkFixture) -> &Path,
    #[case] select_reference_count: fn(&FixtureSummary) -> usize,
) {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();
    let expected_references = select_reference_count(&fixture.summary);

    run_move_operation(select_operation(&fixture)).unwrap();

    assert!(!select_source(&fixture).exists());
    assert!(select_destination(&fixture).exists());
    assert_eq!(
        find_references(select_destination(&fixture), &fixture.root, &NoopProgress)
            .unwrap()
            .len(),
        expected_references
    );
}
