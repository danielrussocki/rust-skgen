const TRACEABILITY: &str = include_str!("../docs/traceability.md");

#[test]
fn traceability_covers_optional_robots_txt_cases() {
    for reference in [
        "tests/discovery_configuration.rs::robots_txt_is_optional_by_default_and_can_be_explicitly_required",
        "tests/cli_arguments.rs::accepts_explicit_robots_txt_requirement_for_create_and_single_update",
        "tests/robots_policy.rs::unavailable_robots_txt_is_optional_unless_required",
        "tests/robots_policy.rs::inaccessible_robots_txt_is_optional_unless_required",
        "tests/robots_policy.rs::malformed_robots_txt_is_optional_unless_required",
        "tests/robots_policy.rs::forbids_a_url_disallowed_for_the_configured_user_agent",
        "tests/managed_skill_metadata.rs::serializes_every_required_configuration_field",
        "tests/managed_skill_metadata.rs::rejects_metadata_without_or_with_an_invalid_robots_requirement",
        "tests/cli_arguments.rs::update_rejects_changes_without_exactly_one_selected_skill",
    ] {
        assert!(
            TRACEABILITY.contains(reference),
            "missing traceability reference: {reference}"
        );
    }
}

#[test]
fn traceability_covers_persisted_robots_requirement_and_unavailable_related_sources() {
    for reference in [
        "tests/cli_integration.rs::update_without_changes_preserves_a_persisted_robots_requirement",
        "src/discover.rs::tests::source_http_error_aborts_discovery_before_any_related_url_is_visited",
        "src/discover.rs::tests::related_http_not_found_is_retained_as_an_unavailable_source",
        "src/discover.rs::tests::related_http_error_other_than_not_found_aborts_without_results",
        "tests/guide_with_references.rs::renders_an_unavailable_related_source_without_presenting_it_as_extracted_content",
        "tests/organized_content.rs::renders_an_unavailable_related_source_as_a_notice",
        "tests/skill_creation_service.rs::creates_a_complete_skill_when_a_related_page_returns_not_found",
        "tests/skill_update_service.rs::updating_a_skill_publishes_an_unavailable_related_page_notice_after_not_found",
        "tests/cli_integration.rs::create_and_update_publish_unavailable_related_documentation_after_http_not_found",
    ] {
        assert!(
            TRACEABILITY.contains(reference),
            "missing traceability reference: {reference}"
        );
    }
}

#[test]
fn traceability_lists_deterministic_coverage_for_every_functional_requirement() {
    for requirement in 1..=8 {
        let clause = format!("RF-{requirement}:");
        assert!(
            TRACEABILITY.contains(&clause),
            "missing traceability coverage for {clause}"
        );
    }
}
