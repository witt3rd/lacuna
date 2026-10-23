//! Agent Skills in `skills/` follow the Anthropic skill-authoring best
//! practices: valid frontmatter (name = directory, ≤64 chars of lowercase
//! letters, digits and hyphens, no reserved words; description non-empty,
//! ≤1024 chars, no XML tags, says when to use it), body under 500 lines,
//! and at least three evaluations in `evaluations.json`.

use std::fs;
use std::path::PathBuf;

fn skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills")
}

fn frontmatter_field<'a>(frontmatter: &'a str, key: &str) -> Option<&'a str> {
    frontmatter
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{key}:")).map(str::trim))
}

#[test]
fn every_skill_follows_authoring_best_practices() {
    let mut checked = 0;
    for entry in fs::read_dir(skills_dir()).expect("skills/ exists") {
        let dir = entry.expect("dir entry").path();
        let skill = dir.join("SKILL.md");
        if !skill.exists() {
            continue;
        }
        let dir_name = dir.file_name().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(&skill).expect("read SKILL.md");
        let rest = text
            .strip_prefix("---\n")
            .unwrap_or_else(|| panic!("{dir_name}: missing frontmatter"));
        let (frontmatter, body) = rest
            .split_once("\n---\n")
            .unwrap_or_else(|| panic!("{dir_name}: unterminated frontmatter"));

        let name =
            frontmatter_field(frontmatter, "name").unwrap_or_else(|| panic!("{dir_name}: no name"));
        assert_eq!(name, dir_name, "name must match the directory");
        assert!(name.len() <= 64, "{name}: name longer than 64 chars");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{name}: name must be lowercase letters, digits, hyphens"
        );
        assert!(
            !name.contains("anthropic") && !name.contains("claude"),
            "{name}: reserved word in name"
        );

        let description = frontmatter_field(frontmatter, "description")
            .unwrap_or_else(|| panic!("{name}: no description"));
        assert!(
            !description.is_empty() && description.len() <= 1024,
            "{name}: description must be 1..=1024 chars"
        );
        assert!(
            !description.contains('<'),
            "{name}: no XML tags in description"
        );
        assert!(
            description.contains("Use when"),
            "{name}: description must say when to use the skill"
        );
        assert!(
            !description.starts_with("I ") && !description.starts_with("You "),
            "{name}: description must be third person"
        );

        assert!(
            body.lines().count() < 500,
            "{name}: SKILL.md body must be under 500 lines"
        );

        let evals: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dir.join("evaluations.json"))
                .unwrap_or_else(|_| panic!("{name}: evaluations.json missing")),
        )
        .expect("valid JSON");
        let evals = evals.as_array().expect("evaluations array");
        assert!(evals.len() >= 3, "{name}: at least three evaluations");
        for e in evals {
            assert!(
                e["skills"]
                    .as_array()
                    .is_some_and(|s| s.iter().any(|x| x == name)),
                "{name}: evaluation must name the skill"
            );
            assert!(
                e["expected_behavior"]
                    .as_array()
                    .is_some_and(|b| !b.is_empty()),
                "{name}: evaluation needs expected_behavior"
            );
        }
        checked += 1;
    }
    assert!(checked >= 5, "expected the lacuna skills, found {checked}");
}
