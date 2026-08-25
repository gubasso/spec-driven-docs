//! Embedded document shelves: list and fetch by short name.
//!
//! A shelf is one embedded directory whose files a reader addresses by stem
//! — optionally with a kind prefix stripped, so `sdd spec distribution`
//! finds `SPEC-distribution.md`. What the shelves contain is `embedded`'s
//! business.

use include_dir::Dir;

/// One addressable shelf of embedded documents.
#[derive(Debug, Clone, Copy)]
pub struct Shelf {
    /// The embedded directory.
    pub dir: &'static Dir<'static>,
    /// A kind prefix hidden from short names, e.g. `SPEC-`.
    pub strip: Option<&'static str>,
}

/// The method chapters and glossary.
pub const METHOD: Shelf = Shelf {
    dir: &crate::embedded::METHOD,
    strip: None,
};
/// The spec documents, addressed without their `SPEC-` prefix.
pub const SPECS: Shelf = Shelf {
    dir: &crate::embedded::SPECS,
    strip: Some("SPEC-"),
};
/// The templates, addressed without their `TEMPLATE-` prefix.
pub const TEMPLATES: Shelf = Shelf {
    dir: &crate::embedded::TEMPLATES,
    strip: Some("TEMPLATE-"),
};
/// The migration guides.
pub const MIGRATIONS: Shelf = Shelf {
    dir: &crate::embedded::MIGRATIONS,
    strip: None,
};

fn short_name<'a>(shelf: &Shelf, file_name: &'a str) -> Option<&'a str> {
    let stem = file_name.strip_suffix(".md")?;
    shelf
        .strip
        .map_or(Some(stem), |prefix| stem.strip_prefix(prefix))
}

/// Every short name the shelf offers, sorted.
#[must_use]
pub fn list(shelf: &Shelf) -> Vec<String> {
    let mut names: Vec<String> = shelf
        .dir
        .files()
        .filter_map(|file| {
            let name = file.path().as_os_str().to_str()?;
            short_name(shelf, name).map(String::from)
        })
        .collect();
    names.sort();
    names
}

/// Fetch one document by short name, full stem, or file name.
#[must_use]
pub fn get(shelf: &Shelf, name: &str) -> Option<&'static str> {
    let wanted = name.strip_suffix(".md").unwrap_or(name);
    shelf.dir.files().find_map(|file| {
        let file_name = file.path().as_os_str().to_str()?;
        let stem = file_name.strip_suffix(".md")?;
        let matches = stem == wanted || short_name(shelf, file_name) == Some(wanted);
        if matches { file.contents_utf8() } else { None }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelves_list_their_short_names() {
        assert!(list(&METHOD).contains(&"glossary".to_string()));
        assert!(list(&SPECS).contains(&"distribution".to_string()));
        assert!(list(&TEMPLATES).contains(&"adr".to_string()));
        assert!(!list(&MIGRATIONS).is_empty());
    }

    #[test]
    fn documents_resolve_by_short_and_full_names() {
        let by_short = get(&SPECS, "distribution").unwrap();
        assert_eq!(get(&SPECS, "SPEC-distribution").unwrap(), by_short);
        assert_eq!(get(&SPECS, "SPEC-distribution.md").unwrap(), by_short);
        assert!(by_short.contains("distribution:instances-operate-offline"));
        assert!(get(&SPECS, "nonexistent").is_none());
    }
}
