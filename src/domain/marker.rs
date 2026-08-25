//! Marker-delimited managed block: string surgery over a host file.
//!
//! The canon owns exactly one region of a consumer's `.pre-commit-config.yaml`
//! — the lines between its BEGIN and END markers — and everything here is a
//! pure function over the host text: validate the markers, strip the region,
//! splice a new one into the `repos:` sequence, and hash what is present.
//! Every line outside the markers passes through byte-identical. What the
//! block contains is the hook renderer's business, and reading or writing
//! the file is the installer's.

use thiserror::Error;

use crate::domain::ownership::Sha256;

/// The line that opens the managed region.
pub const BEGIN: &str = "# BEGIN spec-driven-docs managed";
/// The line that closes the managed region.
pub const END: &str = "# END spec-driven-docs managed";

/// A host file whose markers cannot be trusted, or that cannot host a block.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MarkerError {
    /// Marker counts disagree, or more than one region is present.
    #[error("malformed managed markers in .pre-commit-config.yaml; repair them and re-run")]
    Malformed,
    /// The END marker precedes the BEGIN marker.
    #[error("managed markers are out of order in .pre-commit-config.yaml")]
    OutOfOrder,
    /// The host has no top-level `repos:` sequence to splice into.
    #[error(".pre-commit-config.yaml has no top-level repos: key; add one and re-run")]
    NoReposKey,
}

fn line_content(line: &str) -> &str {
    line.strip_suffix('\n').unwrap_or(line)
}

/// Split a host text into its lines outside the managed region and the
/// region itself, validating the markers first.
///
/// # Errors
///
/// [`MarkerError::Malformed`] when the marker counts disagree or a second
/// region appears; [`MarkerError::OutOfOrder`] when END precedes BEGIN.
pub fn split_block(text: &str) -> Result<(String, Option<String>), MarkerError> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let begins = lines.iter().filter(|l| line_content(l) == BEGIN).count();
    let ends = lines.iter().filter(|l| line_content(l) == END).count();
    if begins != ends || begins > 1 {
        return Err(MarkerError::Malformed);
    }
    if begins == 0 {
        return Ok((text.to_string(), None));
    }
    let first_begin = lines.iter().position(|l| line_content(l) == BEGIN);
    let first_end = lines.iter().position(|l| line_content(l) == END);
    let (Some(begin), Some(end)) = (first_begin, first_end) else {
        return Err(MarkerError::Malformed);
    };
    if begin >= end {
        return Err(MarkerError::OutOfOrder);
    }
    let base: String = lines[..begin].concat() + &lines[end + 1..].concat();
    let block: String = lines[begin..=end].concat();
    Ok((base, Some(block)))
}

fn is_top_level_key(line: &str) -> bool {
    let content = line_content(line);
    let Some(first) = content.bytes().next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    content
        .bytes()
        .position(|b| b == b':')
        .is_some_and(|colon| {
            content
                .bytes()
                .take(colon)
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

fn is_repos_key(line: &str) -> bool {
    let content = line_content(line);
    content == "repos:"
        || (content.starts_with("repos:") && content["repos:".len()..].trim().is_empty())
}

/// The indentation of the first sequence item after `repos:`, or two spaces.
fn item_indent<'a>(lines: &[&'a str], repos_line: usize) -> &'a str {
    lines[repos_line + 1..]
        .iter()
        .find_map(|line| {
            let content = line_content(line);
            let trimmed = content.trim_start();
            trimmed
                .starts_with("- ")
                .then(|| &content[..content.len() - trimmed.len()])
        })
        .filter(|indent| !indent.is_empty())
        .unwrap_or("  ")
}

/// Splice a rendered block (BEGIN through END, newline-terminated) into the
/// `repos:` sequence of a marker-free base, returning the new host text and
/// the indentation the block's entries should have used.
///
/// # Errors
///
/// [`MarkerError::NoReposKey`] when the base has no top-level `repos:` line.
pub fn splice(base: &str, block: &str) -> Result<String, MarkerError> {
    let lines: Vec<&str> = base.split_inclusive('\n').collect();
    let repos_line = lines
        .iter()
        .position(|l| is_repos_key(l))
        .ok_or(MarkerError::NoReposKey)?;
    let end_line = lines[repos_line + 1..]
        .iter()
        .position(|l| is_top_level_key(l))
        .map(|offset| repos_line + 1 + offset);

    let mut out = String::with_capacity(base.len() + block.len());
    if let Some(end) = end_line {
        out.push_str(&lines[..end].concat());
        out.push_str(block);
        out.push_str(&lines[end..].concat());
    } else {
        out.push_str(base);
        if !base.is_empty() && !base.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(block);
    }
    Ok(out)
}

/// The indentation a spliced block's sequence items should carry, measured
/// from the base the block will join.
///
/// # Errors
///
/// [`MarkerError::NoReposKey`] when the base has no top-level `repos:` line.
pub fn splice_indent(base: &str) -> Result<String, MarkerError> {
    let lines: Vec<&str> = base.split_inclusive('\n').collect();
    let repos_line = lines
        .iter()
        .position(|l| is_repos_key(l))
        .ok_or(MarkerError::NoReposKey)?;
    Ok(item_indent(&lines, repos_line).to_string())
}

/// The managed region — BEGIN through END inclusive — as present in a host
/// text, or `None` when no complete region exists.
#[must_use]
pub fn block_region(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let begin = lines.iter().position(|l| line_content(l) == BEGIN)?;
    let end = lines[begin..].iter().position(|l| line_content(l) == END)? + begin;
    Some(lines[begin..=end].concat())
}

/// The hash the manifest records for a host file's managed region.
#[must_use]
pub fn block_hash(text: &str) -> Option<Sha256> {
    block_region(text).map(|region| Sha256::of(region.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str =
        "# BEGIN spec-driven-docs managed\n  - repo: local\n# END spec-driven-docs managed\n";

    #[test]
    fn splits_a_marked_host_and_keeps_outside_bytes() {
        let host = format!("# lead comment\nrepos:\n{BLOCK}  - repo: other\n");
        let (base, block) = split_block(&host).unwrap();
        assert_eq!(base, "# lead comment\nrepos:\n  - repo: other\n");
        assert_eq!(block.as_deref(), Some(BLOCK));
    }

    #[test]
    fn split_without_markers_is_identity() {
        let host = "repos:\n  - repo: other\n";
        let (base, block) = split_block(host).unwrap();
        assert_eq!(base, host);
        assert!(block.is_none());
    }

    #[test]
    fn lone_begin_is_malformed() {
        let host = "repos:\n# BEGIN spec-driven-docs managed\n  - repo: local\n";
        assert_eq!(split_block(host), Err(MarkerError::Malformed));
    }

    #[test]
    fn lone_end_is_malformed() {
        let host = "repos:\n# END spec-driven-docs managed\n";
        assert_eq!(split_block(host), Err(MarkerError::Malformed));
    }

    #[test]
    fn duplicate_regions_are_malformed() {
        let host = format!("repos:\n{BLOCK}{BLOCK}");
        assert_eq!(split_block(&host), Err(MarkerError::Malformed));
    }

    #[test]
    fn reversed_markers_are_out_of_order() {
        let host = "repos:\n# END spec-driven-docs managed\n# BEGIN spec-driven-docs managed\n";
        assert_eq!(split_block(host), Err(MarkerError::OutOfOrder));
    }

    #[test]
    fn splices_before_the_next_top_level_key() {
        let base = "repos:\n  - repo: other\nci:\n  autofix: true\n";
        let out = splice(base, BLOCK).unwrap();
        assert_eq!(
            out,
            format!("repos:\n  - repo: other\n{BLOCK}ci:\n  autofix: true\n")
        );
    }

    #[test]
    fn splices_at_eof_when_repos_is_last() {
        let base = "default_stages: [pre-commit]\nrepos:\n  - repo: other\n";
        let out = splice(base, BLOCK).unwrap();
        assert_eq!(out, format!("{base}{BLOCK}"));
    }

    #[test]
    fn refuses_a_base_with_no_repos_key() {
        assert_eq!(
            splice("ci:\n  autofix: true\n", BLOCK),
            Err(MarkerError::NoReposKey)
        );
        assert_eq!(splice("  repos:\n", BLOCK), Err(MarkerError::NoReposKey));
    }

    #[test]
    fn measures_item_indent_from_the_first_entry() {
        assert_eq!(
            splice_indent("repos:\n    - repo: other\n").unwrap(),
            "    "
        );
        assert_eq!(splice_indent("repos:\n").unwrap(), "  ");
    }

    #[test]
    fn strip_then_splice_round_trips_outside_comments() {
        let host = format!(
            "# above\nrepos:\n  # inside, before ours\n  - repo: other # trailing\n{BLOCK}ci:\n  # below\n"
        );
        let (base, _) = split_block(&host).unwrap();
        let out = splice(&base, BLOCK).unwrap();
        assert_eq!(out, host);
    }

    #[test]
    fn hashes_the_inclusive_region() {
        let host = format!("repos:\n{BLOCK}");
        assert_eq!(block_hash(&host), Some(Sha256::of(BLOCK.as_bytes())));
        assert_eq!(block_hash("repos:\n"), None);
    }
}
