//! What a reader does after a pin move.
//!
//! The moved text says how the instance follows a moved pin: through the
//! setup skill of the sdd that the devshell now provides, never through a
//! host copy or the older skill the project still holds. One constant holds
//! the text, and the specs, the skill, and the release guide quote it.

/// The name of the skill that runs the upgrade, as a literal for
/// `concat!`, so a rename changes one line.
macro_rules! setup_skill {
    () => {
        "sdd-setup"
    };
}

/// The skill that runs an upgrade.
pub const SETUP_SKILL: &str = setup_skill!();

/// The command that prints the setup skill the binary embeds.
pub const SKILL_SHOW: &str = concat!("sdd skill show ", setup_skill!());

/// What a reader does after the pin moved, or while the instance trails
/// the binary.
///
/// SATISFIES setup:a-pin-move-names-the-skill-route
pub const MOVED: &str = concat!(
    "After the pin move, load the devshell and run the upgrade through the ",
    setup_skill!(),
    " skill of the sdd in that devshell, and do every migration it names. Before `sdd upgrade`, read it with `sdd skill show ",
    setup_skill!(),
    "`. After it, the project's .claude/skills/",
    setup_skill!(),
    " is that skill. Never use a host copy or an older skill."
);

#[cfg(test)]
mod tests {
    use super::*;

    /// VERIFIES setup:a-pin-move-names-the-skill-route
    #[test]
    fn the_moved_text_names_the_skill_and_its_show_command() {
        assert!(MOVED.contains(SKILL_SHOW));
        assert!(MOVED.contains(&format!("the {SETUP_SKILL} skill of the sdd")));
        assert!(MOVED.contains(&format!(".claude/skills/{SETUP_SKILL} is that skill")));
    }
}
