use rust_skgen::{
    cli::{CommandOutput, SkillOutput, write_command_output, write_invalid_argument},
    domain::SkillName,
};

#[test]
fn create_success_is_written_to_standard_output_with_exit_code_zero() {
    let mut standard_output = Vec::new();
    let mut standard_error = Vec::new();

    let exit_code = write_command_output(
        CommandOutput::Created(SkillName::parse("example-skill").unwrap()),
        &mut standard_output,
        &mut standard_error,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
    assert_eq!(
        String::from_utf8(standard_output).unwrap(),
        "Created skill: example-skill\n"
    );
    assert!(standard_error.is_empty());
}

#[test]
fn create_failure_is_written_to_standard_error_with_exit_code_one() {
    let mut standard_output = Vec::new();
    let mut standard_error = Vec::new();

    let exit_code = write_command_output(
        CommandOutput::CreateFailed {
            name: SkillName::parse("failed-skill").unwrap(),
            reason: "the source returned HTTP status 500".to_owned(),
        },
        &mut standard_output,
        &mut standard_error,
    )
    .unwrap();

    assert_eq!(exit_code, 1);
    assert!(standard_output.is_empty());
    assert_eq!(
        String::from_utf8(standard_error).unwrap(),
        "Failed skill: failed-skill: the source returned HTTP status 500\n"
    );
}

#[test]
fn update_writes_each_skill_result_and_returns_failure_when_any_skill_fails() {
    let mut standard_output = Vec::new();
    let mut standard_error = Vec::new();

    let exit_code = write_command_output(
        CommandOutput::Updated(vec![
            SkillOutput::Updated(SkillName::parse("updated-skill").unwrap()),
            SkillOutput::Failed {
                name: SkillName::parse("failed-skill").unwrap(),
                reason: "the source returned HTTP status 500".to_owned(),
            },
        ]),
        &mut standard_output,
        &mut standard_error,
    )
    .unwrap();

    assert_eq!(exit_code, 1);
    assert_eq!(
        String::from_utf8(standard_output).unwrap(),
        "Updated skill: updated-skill\n"
    );
    assert_eq!(
        String::from_utf8(standard_error).unwrap(),
        "Failed skill: failed-skill: the source returned HTTP status 500\n"
    );
}

#[test]
fn no_managed_skills_is_a_successful_standard_output_message() {
    let mut standard_output = Vec::new();
    let mut standard_error = Vec::new();

    let exit_code = write_command_output(
        CommandOutput::NoManagedSkills,
        &mut standard_output,
        &mut standard_error,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
    assert_eq!(
        String::from_utf8(standard_output).unwrap(),
        "No managed skills found.\n"
    );
    assert!(standard_error.is_empty());
}

#[test]
fn invalid_arguments_are_written_to_standard_error_with_exit_code_two() {
    let mut standard_output = Vec::new();
    let mut standard_error = Vec::new();

    let exit_code = write_invalid_argument(
        "configuration changes require exactly one selected skill",
        &mut standard_output,
        &mut standard_error,
    )
    .unwrap();

    assert_eq!(exit_code, 2);
    assert!(standard_output.is_empty());
    assert_eq!(
        String::from_utf8(standard_error).unwrap(),
        "Invalid argument: configuration changes require exactly one selected skill\n"
    );
}
