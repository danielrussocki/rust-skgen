const TRACEABILITY: &str = include_str!("../docs/traceability.md");

#[test]
fn traceability_references_existing_tests() {
    for reference in TRACEABILITY
        .split('`')
        .filter(|value| value.contains(".rs::"))
    {
        let Some((path, test_name)) = reference.split_once(".rs::") else {
            continue;
        };
        let source_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("{path}.rs"));
        let test_name = test_name.trim_start_matches("tests::");
        let source = std::fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_path.display()));
        assert!(
            source.contains(&format!("fn {test_name}")),
            "traceability reference does not identify an existing test: {reference}"
        );
    }
}

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
fn traceability_covers_related_non_html_responses() {
    for reference in [
        "src/discover.rs::tests::related_non_html_response_is_skipped_while_remaining_html_pages_are_discovered",
        "tests/skill_creation_service.rs::creates_a_skill_while_skipping_a_related_non_html_response",
        "tests/skill_update_service.rs::updating_a_skill_skips_a_related_non_html_response",
        "tests/cli_integration.rs::create_and_update_skip_related_non_html_documentation",
    ] {
        assert!(
            TRACEABILITY.contains(reference),
            "missing traceability reference: {reference}"
        );
    }
}

#[test]
fn traceability_lists_deterministic_coverage_for_every_functional_requirement() {
    for requirement in 1..=12 {
        let clause = format!("RF-{requirement}:");
        assert!(
            TRACEABILITY.contains(&clause),
            "missing traceability coverage for {clause}"
        );
    }
}

#[test]
fn traceability_covers_deterministic_link_prioritization() {
    for reference in [
        "tests/link_candidate.rs::scores_documentation_signals_case_insensitively_from_each_context",
        "tests/link_candidate.rs::penalizes_terms_and_resource_destinations_without_rejecting_candidates",
        "src/discover.rs::tests::prioritizes_html_candidates_before_fetching_and_breaks_ties_by_canonical_url",
        "tests/sitemap_discovery.rs::prioritizes_sitemap_candidates_before_html_candidates",
        "src/discover.rs::tests::limited_traversal_uses_priority_for_mixed_signals_without_relaxing_scope",
    ] {
        assert!(
            TRACEABILITY.contains(reference),
            "missing traceability reference: {reference}"
        );
    }
}

#[test]
fn traceability_covers_related_redirects() {
    for reference in [
        "src/discover.rs::tests::source_redirect_aborts_discovery_before_related_pages_are_visited",
        "src/discover.rs::tests::related_redirect_is_skipped_while_remaining_html_pages_are_discovered",
        "src/discover.rs::tests::extractable_source_is_published_when_all_related_urls_redirect",
        "tests/skill_creation_service.rs::creates_a_skill_while_skipping_a_related_redirect",
        "tests/skill_update_service.rs::updating_a_skill_skips_a_related_redirect",
        "tests/cli_integration.rs::create_and_update_skip_related_redirects_without_following_them",
    ] {
        assert!(
            TRACEABILITY.contains(reference),
            "missing traceability reference: {reference}"
        );
    }
}
