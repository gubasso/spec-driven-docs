//! Embedded document shelves: list and fetch by short name.
//!
//! A shelf is one embedded directory whose files a reader addresses by stem
//! — optionally with a kind prefix stripped, so `sdd spec distribution`
//! finds `SPEC-distribution.md`. What the shelves contain is `embedded`'s
//! business.

use include_dir::Dir;

use crate::domain::docs_catalog::ShelfId;

/// One addressable shelf of embedded documents.
#[derive(Debug, Clone, Copy)]
pub struct Shelf {
    /// Which shelf this is, as the catalog names it.
    pub id: ShelfId,
    /// The embedded directory.
    pub dir: &'static Dir<'static>,
    /// Whether the shelf serves the merged spec set instead of `dir` alone.
    pub merged_specs: bool,
    /// A kind prefix hidden from short names, e.g. `SPEC-`.
    pub strip: Option<&'static str>,
}

/// The method chapters and glossary.
pub const METHOD: Shelf = Shelf {
    id: ShelfId::Method,
    dir: &crate::embedded::METHOD,
    strip: None,
    merged_specs: false,
};
/// The spec documents, addressed without their `SPEC-` prefix.
pub const SPECS: Shelf = Shelf {
    id: ShelfId::Spec,
    dir: &crate::embedded::SPECS,
    strip: Some("SPEC-"),
    merged_specs: true,
};
/// The templates, addressed without their `TEMPLATE-` prefix.
pub const TEMPLATES: Shelf = Shelf {
    id: ShelfId::Template,
    dir: &crate::embedded::TEMPLATES,
    strip: Some("TEMPLATE-"),
    merged_specs: false,
};

impl Shelf {
    /// Every file the shelf serves, by file name.
    fn files(&self) -> Vec<(&'static str, &'static include_dir::File<'static>)> {
        if self.merged_specs {
            return crate::embedded::spec_files();
        }
        self.dir
            .files()
            .filter_map(|file| Some((file.path().as_os_str().to_str()?, file)))
            .collect()
    }
}

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
        .files()
        .into_iter()
        .filter_map(|(name, _)| short_name(shelf, name).map(String::from))
        .collect();
    names.sort();
    names
}

/// Fetch one document by short name, full stem, or file name.
#[must_use]
pub fn get(shelf: &Shelf, name: &str) -> Option<&'static str> {
    let wanted = name.strip_suffix(".md").unwrap_or(name);
    shelf.files().into_iter().find_map(|(file_name, file)| {
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
