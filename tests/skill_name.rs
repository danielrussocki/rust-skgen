use rust_skgen::domain::{SkillName, SkillNameError};

#[test]
fn accepts_valid_skill_name_slugs() {
    for value in ["radix", "radix-ui", "skill-123", "123"] {
        let skill_name = SkillName::parse(value);

        assert!(skill_name.is_ok(), "expected valid slug: {value}");
    }
}

#[test]
fn accepts_a_skill_name_with_sixty_four_characters() {
    let value = "a".repeat(64);

    assert!(SkillName::parse(value).is_ok());
}

#[test]
fn rejects_an_empty_skill_name() {
    assert_eq!(SkillName::parse(""), Err(SkillNameError::Empty));
}

#[test]
fn rejects_skill_names_with_invalid_characters() {
    for value in ["Radix", "radix_ui", "radix ui", "radix-ñ"] {
        assert!(
            matches!(
                SkillName::parse(value),
                Err(SkillNameError::InvalidCharacter(_))
            ),
            "expected invalid character error: {value}"
        );
    }
}

#[test]
fn rejects_skill_names_that_start_or_end_with_a_hyphen() {
    assert_eq!(
        SkillName::parse("-radix"),
        Err(SkillNameError::StartsWithHyphen)
    );
    assert_eq!(
        SkillName::parse("radix-"),
        Err(SkillNameError::EndsWithHyphen)
    );
}

#[test]
fn rejects_skill_names_longer_than_sixty_four_characters() {
    let value = "a".repeat(65);

    assert_eq!(
        SkillName::parse(value),
        Err(SkillNameError::TooLong { length: 65 })
    );
}
