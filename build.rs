//! Build script: embedded-asset change tracking only.
//!
//! `include_dir!` embeds at compile time but cargo does not watch a
//! directory for new or deleted files on its own; naming each embedded
//! root here makes any change under them rebuild the crate. The roots
//! come from `src/payload_roots.rs`, pasted in verbatim, so this script
//! and the binary cannot disagree about what the payload is. No code
//! generation happens.

include!("src/payload_roots.rs");

fn main() {
    for dir in PAYLOAD_ROOTS.iter().copied().chain([
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-CC-BY-4.0",
        "THIRD_PARTY_NOTICES.md",
    ]) {
        println!("cargo:rerun-if-changed={dir}");
    }
    println!("cargo:rerun-if-changed=src/payload_roots.rs");
}
