//! Build script: embedded-asset change tracking only.
//!
//! `include_dir!` embeds at compile time but cargo does not watch a
//! directory for new or deleted files on its own; naming each embedded
//! root here makes any change under them rebuild the crate. No code
//! generation happens.

fn main() {
    for dir in [
        "_docs/specs",
        "templates",
        ".markdownlint",
        "instance/snippets",
        "method",
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-CC-BY-4.0",
    ] {
        println!("cargo:rerun-if-changed={dir}");
    }
}
