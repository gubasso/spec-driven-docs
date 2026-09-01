# Release Setup Sources

External sources behind [release-setup.md](../guides/release-setup.md): what the registry's token form carries and where cargo keeps the credential the bootstrap publish uses. Each entry states what the source says and which step it bears on.

Verified against the listed sources on 2026-09-01.

## The crates.io token form, from its own source

The New API Token page (`crates.io/settings/tokens`, New Token) carries four fields. Name is required text. Expiration is a dropdown of No expiration, 7, 30, 60, 90, and 365 days, plus a custom date, defaulting to 90; 7 days is the shortest preset. Scopes are five checkboxes — `change-owners`, `publish-new`, `publish-update`, `trusted-publishing`, `yank` — of which at least one must be checked. Crates is a pattern list whose empty state reads Unrestricted; Add pattern appends an entry, and RFC 2947 states a pattern allows interacting with matching crates published after token creation, which is what lets a bootstrap token pin a crate name that does not exist yet. Generate Token returns to the token list, which shows the value once beside the explainer "Make sure to copy your API token now. You won't be able to see it again!"; the copy icon renders only where the browser exposes a clipboard, reports "Copied to clipboard!" on success and "Copy to clipboard failed!" on failure, and the shown value is selectable for a hand copy. A token row lists the name, Scopes, Crates, and an Expires distance.

- <https://github.com/rust-lang/crates.io/blob/main/svelte/src/routes/settings/tokens/new/+page.svelte>
- <https://github.com/rust-lang/crates.io/blob/main/svelte/src/lib/utils/token-scopes.ts>
- <https://github.com/rust-lang/crates.io/blob/main/svelte/src/routes/settings/tokens/+page.svelte>
- <https://github.com/rust-lang/crates.io/blob/main/svelte/src/lib/components/CopyButton.svelte>
- <https://rust-lang.github.io/rfcs/2947-crates-io-token-scopes.html>

Bearing: release-setup.md step 6.2, every field the token step names.

## The Cargo book, on the token's life around the publish

Publishing requires a crates.io account with a verified email. `cargo login` prompts for the token on standard input and stores it in `$CARGO_HOME/credentials.toml`; the book marks the token a secret to revoke immediately if it leaks, and `cargo logout` removes the stored copy.

- <https://doc.rust-lang.org/cargo/reference/publishing.html>

Bearing: release-setup.md steps 6.3 and 8, the login and the revocation.
