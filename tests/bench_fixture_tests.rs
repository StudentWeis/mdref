#[allow(dead_code)]
#[path = "../benches/support/mod.rs"]
mod support;

use mdref::{LinkType, find_links, find_references};
use support::{FixtureProfile, build_fixture};

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
    assert!(fixture.hot_file.exists());
    assert!(fixture.hot_directory.exists());
    assert!(fixture.representative_document.exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_fixture_reference_counts_match_summary() {
    let fixture = build_fixture(FixtureProfile::Small).unwrap();

    let hot_file_references = find_references(&fixture.hot_file, &fixture.root).unwrap();
    let bundle_references = find_references(&fixture.hot_directory, &fixture.root).unwrap();

    assert_eq!(
        hot_file_references.len(),
        fixture.summary.hot_file_references
    );
    assert_eq!(
        bundle_references.len(),
        fixture.summary.bundle_directory_references
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
