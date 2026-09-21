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
