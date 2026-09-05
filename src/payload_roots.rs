// The payload inventory: every authored root the binary embeds.
//
// One declaration, three readers. `embedded` must embed each root and a
// unit test there holds its statics to this list; `build.rs` `include!`s
// this file to emit change tracking per root; and the canon suite scans
// exactly these roots for what the payload may not carry, so a root added
// without a line here would ship unscanned.
//
// `build.rs` pastes this file verbatim, so it holds items and plain
// comments only — no `use`, no inner doc comments, no other modules.

/// Every authored root the binary embeds, in one place.
pub const PAYLOAD_ROOTS: [&str; 8] = [
    "_docs/specs",
    "templates",
    ".markdownlint",
    "instance/snippets",
    "method",
    "skills",
    "skill-shared",
    "third-party/simpleenglish",
];
