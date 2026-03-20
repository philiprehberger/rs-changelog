//! Programmatic CHANGELOG.md parsing, generation, and manipulation
//! following the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
//!
//! # Usage
//!
//! ```rust
//! use philiprehberger_changelog::{Changelog, Category};
//!
//! let md = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- New feature\n\n## [0.1.0] - 2026-03-19\n\n### Fixed\n\n- Bug fix\n";
//! let mut changelog = Changelog::parse(md).unwrap();
//!
//! changelog.add_entry("Unreleased", Category::Fixed, "Another fix");
//! changelog.release("0.2.0", "2026-03-20");
//!
//! let output = changelog.to_markdown();
//! ```

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// The standard category ordering used for rendering.
const CATEGORY_ORDER: [Category; 6] = [
    Category::Added,
    Category::Changed,
    Category::Deprecated,
    Category::Removed,
    Category::Fixed,
    Category::Security,
];

/// A changelog entry category following Keep a Changelog conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// New features.
    Added,
    /// Changes in existing functionality.
    Changed,
    /// Soon-to-be removed features.
    Deprecated,
    /// Removed features.
    Removed,
    /// Bug fixes.
    Fixed,
    /// Vulnerability fixes.
    Security,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Category::Added => "Added",
            Category::Changed => "Changed",
            Category::Deprecated => "Deprecated",
            Category::Removed => "Removed",
            Category::Fixed => "Fixed",
            Category::Security => "Security",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Category {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "added" => Ok(Category::Added),
            "changed" => Ok(Category::Changed),
            "deprecated" => Ok(Category::Deprecated),
            "removed" => Ok(Category::Removed),
            "fixed" => Ok(Category::Fixed),
            "security" => Ok(Category::Security),
            _ => Err(ParseError::InvalidCategory(s.to_string())),
        }
    }
}

/// A single changelog entry with a category and description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The category this entry belongs to.
    pub category: Category,
    /// The description text of the entry.
    pub description: String,
}

/// A version section in the changelog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// The version name, e.g. `"0.1.0"` or `"Unreleased"`.
    pub name: String,
    /// The release date in `YYYY-MM-DD` format, or `None` for Unreleased.
    pub date: Option<String>,
    /// The entries in this version section.
    pub entries: Vec<Entry>,
}

impl Version {
    /// Returns `true` if this is the Unreleased section.
    pub fn is_unreleased(&self) -> bool {
        self.name.eq_ignore_ascii_case("unreleased")
    }

    /// Returns entries matching the given category.
    pub fn entries_by_category(&self, category: Category) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Adds an entry to this version.
    pub fn add_entry(&mut self, category: Category, description: impl Into<String>) {
        self.entries.push(Entry {
            category,
            description: description.into(),
        });
    }
}

/// Errors that can occur during changelog parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input string was empty.
    EmptyInput,
    /// The input format was invalid.
    InvalidFormat(String),
    /// An unrecognized category was encountered.
    InvalidCategory(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyInput => write!(f, "input is empty"),
            ParseError::InvalidFormat(msg) => write!(f, "invalid format: {}", msg),
            ParseError::InvalidCategory(cat) => write!(f, "invalid category: {}", cat),
        }
    }
}

impl std::error::Error for ParseError {}

/// A parsed changelog document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changelog {
    /// The title of the changelog (usually `"Changelog"`).
    pub title: String,
    /// Optional preamble/description text after the title.
    pub description: Option<String>,
    /// The version sections, ordered from newest to oldest.
    pub versions: Vec<Version>,
}

impl Changelog {
    /// Parse a Keep a Changelog formatted markdown string.
    ///
    /// Supports header formats:
    /// - `## [version] - date`
    /// - `## [version]`
    /// - `## version (date)`
    /// - `## version - date`
    pub fn parse(markdown: &str) -> Result<Self, ParseError> {
        let trimmed = markdown.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let lines: Vec<&str> = trimmed.lines().collect();

        // Find the title (first `# ` line)
        let title_idx = lines
            .iter()
            .position(|l| l.starts_with("# ") && !l.starts_with("## "))
            .ok_or_else(|| {
                ParseError::InvalidFormat("no title heading found".to_string())
            })?;

        let title = lines[title_idx].trim_start_matches("# ").trim().to_string();

        // Collect description lines (between title and first ## heading)
        let mut description_lines: Vec<&str> = Vec::new();
        let mut idx = title_idx + 1;
        while idx < lines.len() {
            if lines[idx].starts_with("## ") {
                break;
            }
            description_lines.push(lines[idx]);
            idx += 1;
        }

        let description_text = description_lines.join("\n").trim().to_string();
        let description = if description_text.is_empty() {
            None
        } else {
            Some(description_text)
        };

        // Parse version sections
        let mut versions: Vec<Version> = Vec::new();
        let mut current_version: Option<Version> = None;
        let mut current_category: Option<Category> = None;

        for line in lines.iter().skip(idx) {

            if line.starts_with("## ") {
                // Save previous version
                if let Some(v) = current_version.take() {
                    versions.push(v);
                }
                current_category = None;

                let header = line.trim_start_matches("## ").trim();
                let (name, date) = parse_version_header(header);
                current_version = Some(Version {
                    name,
                    date,
                    entries: Vec::new(),
                });
            } else if line.starts_with("### ") {
                let cat_str = line.trim_start_matches("### ").trim();
                match Category::from_str(cat_str) {
                    Ok(cat) => current_category = Some(cat),
                    Err(_) => {
                        return Err(ParseError::InvalidCategory(cat_str.to_string()));
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("- ") {
                if let (Some(ref mut ver), Some(cat)) =
                    (&mut current_version, current_category)
                {
                    ver.entries.push(Entry {
                        category: cat,
                        description: stripped.trim().to_string(),
                    });
                }
            } else if !line.trim().is_empty() {
                // Multi-line continuation: append to previous entry
                if let Some(ref mut ver) = current_version {
                    if let Some(last_entry) = ver.entries.last_mut() {
                        last_entry.description.push(' ');
                        last_entry.description.push_str(line.trim());
                    }
                }
            }
        }

        // Save last version
        if let Some(v) = current_version.take() {
            versions.push(v);
        }

        Ok(Changelog {
            title,
            description,
            versions,
        })
    }

    /// Add an entry to the specified version. Creates the version if it
    /// does not already exist. If `version` is `"Unreleased"`, adds to (or
    /// creates) the Unreleased section at the top.
    pub fn add_entry(
        &mut self,
        version: &str,
        category: Category,
        description: impl Into<String>,
    ) {
        let desc = description.into();

        if let Some(ver) = self.versions.iter_mut().find(|v| v.name == version) {
            ver.add_entry(category, desc);
        } else {
            let is_unreleased = version.eq_ignore_ascii_case("unreleased");
            let new_version = Version {
                name: version.to_string(),
                date: None,
                entries: vec![Entry {
                    category,
                    description: desc,
                }],
            };
            if is_unreleased {
                self.versions.insert(0, new_version);
            } else {
                self.versions.push(new_version);
            }
        }
    }

    /// Promote the Unreleased section to a named version with the given date,
    /// then create a new empty Unreleased section at the top.
    pub fn release(&mut self, version: &str, date: &str) {
        if let Some(unreleased) = self.versions.iter_mut().find(|v| v.is_unreleased()) {
            unreleased.name = version.to_string();
            unreleased.date = Some(date.to_string());
        }

        // Insert new empty Unreleased at the top
        self.versions.insert(
            0,
            Version {
                name: "Unreleased".to_string(),
                date: None,
                entries: Vec::new(),
            },
        );
    }

    /// Find a version by name.
    pub fn get_version(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|v| v.name == name)
    }

    /// Returns a slice of all versions.
    pub fn versions(&self) -> &[Version] {
        &self.versions
    }

    /// Returns the first non-Unreleased version (the latest release).
    pub fn latest_version(&self) -> Option<&Version> {
        self.versions.iter().find(|v| !v.is_unreleased())
    }

    /// Returns the Unreleased section, if present.
    pub fn unreleased(&self) -> Option<&Version> {
        self.versions.iter().find(|v| v.is_unreleased())
    }

    /// Render the changelog back to well-formatted Markdown.
    ///
    /// Entries are grouped by category in the standard order:
    /// Added, Changed, Deprecated, Removed, Fixed, Security.
    /// Categories with no entries are omitted.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# {}\n", self.title));

        if let Some(ref desc) = self.description {
            out.push('\n');
            out.push_str(desc);
            out.push('\n');
        }

        for version in &self.versions {
            out.push('\n');
            if version.is_unreleased() {
                out.push_str("## [Unreleased]\n");
            } else if let Some(ref date) = version.date {
                out.push_str(&format!("## [{}] - {}\n", version.name, date));
            } else {
                out.push_str(&format!("## [{}]\n", version.name));
            }

            // Group entries by category
            let mut by_category: HashMap<Category, Vec<&Entry>> = HashMap::new();
            for entry in &version.entries {
                by_category
                    .entry(entry.category)
                    .or_default()
                    .push(entry);
            }

            for cat in &CATEGORY_ORDER {
                if let Some(entries) = by_category.get(cat) {
                    out.push_str(&format!("\n### {}\n\n", cat));
                    for entry in entries {
                        out.push_str(&format!("- {}\n", entry.description));
                    }
                }
            }
        }

        out
    }

    /// Validate the changelog for compliance issues.
    ///
    /// Returns a list of human-readable issue descriptions.
    /// An empty list means the changelog is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check dates are YYYY-MM-DD
        for version in &self.versions {
            if let Some(ref date) = version.date {
                if !is_valid_date(date) {
                    issues.push(format!(
                        "Version {} has invalid date format: {} (expected YYYY-MM-DD)",
                        version.name, date
                    ));
                }
            }

            // Check entries have non-empty descriptions
            for entry in &version.entries {
                if entry.description.trim().is_empty() {
                    issues.push(format!(
                        "Version {} has an empty {} entry",
                        version.name, entry.category
                    ));
                }
            }
        }

        // Check version ordering (descending semver, skip Unreleased)
        let semver_versions: Vec<&Version> = self
            .versions
            .iter()
            .filter(|v| !v.is_unreleased())
            .collect();

        for window in semver_versions.windows(2) {
            let a = &window[0].name;
            let b = &window[1].name;
            if let (Some(va), Some(vb)) = (parse_semver(a), parse_semver(b)) {
                if va <= vb {
                    issues.push(format!(
                        "Versions are not in descending order: {} should come after {}",
                        a, b
                    ));
                }
            }
        }

        issues
    }

    /// Return entries that exist in `v2` but not in `v1`.
    ///
    /// Returns `None` if either version is not found.
    pub fn diff(&self, v1: &str, v2: &str) -> Option<Vec<Entry>> {
        let ver1 = self.get_version(v1)?;
        let ver2 = self.get_version(v2)?;

        let v1_entries: Vec<(&Category, &String)> = ver1
            .entries
            .iter()
            .map(|e| (&e.category, &e.description))
            .collect();

        let diff: Vec<Entry> = ver2
            .entries
            .iter()
            .filter(|e| !v1_entries.contains(&(&e.category, &e.description)))
            .cloned()
            .collect();

        Some(diff)
    }
}

/// Parse a version header string into (name, optional date).
fn parse_version_header(header: &str) -> (String, Option<String>) {
    // Format: [version] - date
    if header.starts_with('[') {
        if let Some(end_bracket) = header.find(']') {
            let name = header[1..end_bracket].trim().to_string();
            let rest = header[end_bracket + 1..].trim();
            let date = if rest.starts_with('-') {
                let d = rest.trim_start_matches('-').trim();
                if d.is_empty() {
                    None
                } else {
                    Some(d.to_string())
                }
            } else {
                None
            };
            return (name, date);
        }
    }

    // Format: version (date)
    if let Some(paren_start) = header.find('(') {
        if let Some(paren_end) = header.find(')') {
            let name = header[..paren_start].trim().to_string();
            let date = header[paren_start + 1..paren_end].trim().to_string();
            return (name, Some(date));
        }
    }

    // Format: version - date
    if let Some(dash_pos) = header.find(" - ") {
        let name = header[..dash_pos].trim().to_string();
        let date = header[dash_pos + 3..].trim().to_string();
        return (name, Some(date));
    }

    // Just a version name
    (header.trim().to_string(), None)
}

/// Check if a date string matches YYYY-MM-DD format.
fn is_valid_date(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts[0].chars().all(|c| c.is_ascii_digit())
        && parts[1].chars().all(|c| c.is_ascii_digit())
        && parts[2].chars().all(|c| c.is_ascii_digit())
}

/// Parse a version string as (major, minor, patch). Returns None if not valid semver.
fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts[2].parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CHANGELOG: &str = "\
# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- Upcoming feature

## [0.2.0] - 2026-03-15

### Added

- New widget

### Fixed

- Crash on startup

## [0.1.0] - 2026-03-01

### Added

- Initial release
";

    #[test]
    fn test_parse_basic() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        assert_eq!(changelog.title, "Changelog");
        assert!(changelog.description.is_some());
        assert_eq!(changelog.versions.len(), 3);

        let unreleased = &changelog.versions[0];
        assert!(unreleased.is_unreleased());
        assert_eq!(unreleased.entries.len(), 1);

        let v020 = &changelog.versions[1];
        assert_eq!(v020.name, "0.2.0");
        assert_eq!(v020.date.as_deref(), Some("2026-03-15"));
        assert_eq!(v020.entries.len(), 2);
    }

    #[test]
    fn test_parse_multiple_versions() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        assert_eq!(changelog.versions.len(), 3);
        assert_eq!(changelog.versions[1].name, "0.2.0");
        assert_eq!(changelog.versions[2].name, "0.1.0");
    }

    #[test]
    fn test_parse_bracket_dash_format() {
        let md = "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n### Added\n\n- Feature\n";
        let changelog = Changelog::parse(md).unwrap();
        assert_eq!(changelog.versions[0].name, "1.0.0");
        assert_eq!(changelog.versions[0].date.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn test_parse_paren_format() {
        let md = "# Changelog\n\n## 1.0.0 (2026-01-01)\n\n### Fixed\n\n- Bug\n";
        let changelog = Changelog::parse(md).unwrap();
        assert_eq!(changelog.versions[0].name, "1.0.0");
        assert_eq!(changelog.versions[0].date.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn test_parse_plain_dash_format() {
        let md = "# Changelog\n\n## 1.0.0 - 2026-01-01\n\n### Added\n\n- Feature\n";
        let changelog = Changelog::parse(md).unwrap();
        assert_eq!(changelog.versions[0].name, "1.0.0");
        assert_eq!(changelog.versions[0].date.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn test_add_entry_to_unreleased() {
        let mut changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        changelog.add_entry("Unreleased", Category::Fixed, "A bug fix");
        let unreleased = changelog.unreleased().unwrap();
        assert_eq!(unreleased.entries.len(), 2);
        assert_eq!(unreleased.entries[1].category, Category::Fixed);
        assert_eq!(unreleased.entries[1].description, "A bug fix");
    }

    #[test]
    fn test_add_entry_creates_version() {
        let mut changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        changelog.add_entry("0.3.0", Category::Added, "Something new");
        let ver = changelog.get_version("0.3.0").unwrap();
        assert_eq!(ver.entries.len(), 1);
        assert_eq!(ver.entries[0].description, "Something new");
    }

    #[test]
    fn test_add_entry_creates_unreleased() {
        let md = "# Changelog\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- Feature\n";
        let mut changelog = Changelog::parse(md).unwrap();
        assert!(changelog.unreleased().is_none());
        changelog.add_entry("Unreleased", Category::Added, "New thing");
        assert!(changelog.unreleased().is_some());
        // Unreleased should be at the top
        assert!(changelog.versions[0].is_unreleased());
    }

    #[test]
    fn test_release_promotes_unreleased() {
        let mut changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        changelog.release("0.3.0", "2026-03-20");

        // New Unreleased should be at top
        assert!(changelog.versions[0].is_unreleased());
        assert!(changelog.versions[0].entries.is_empty());

        // Old Unreleased should now be 0.3.0
        let released = changelog.get_version("0.3.0").unwrap();
        assert_eq!(released.date.as_deref(), Some("2026-03-20"));
        assert_eq!(released.entries.len(), 1);
        assert_eq!(released.entries[0].description, "Upcoming feature");
    }

    #[test]
    fn test_to_markdown_roundtrip() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let rendered = changelog.to_markdown();
        let reparsed = Changelog::parse(&rendered).unwrap();
        assert_eq!(changelog.title, reparsed.title);
        assert_eq!(changelog.versions.len(), reparsed.versions.len());
        for (orig, re) in changelog.versions.iter().zip(reparsed.versions.iter()) {
            assert_eq!(orig.name, re.name);
            assert_eq!(orig.date, re.date);
            assert_eq!(orig.entries.len(), re.entries.len());
            for (oe, re_entry) in orig.entries.iter().zip(re.entries.iter()) {
                assert_eq!(oe.category, re_entry.category);
                assert_eq!(oe.description, re_entry.description);
            }
        }
    }

    #[test]
    fn test_validate_bad_date() {
        let md = "# Changelog\n\n## [0.1.0] - 2026/03/19\n\n### Added\n\n- Feature\n";
        let changelog = Changelog::parse(md).unwrap();
        let issues = changelog.validate();
        assert!(!issues.is_empty());
        assert!(issues[0].contains("invalid date format"));
    }

    #[test]
    fn test_validate_valid_changelog() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let issues = changelog.validate();
        assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
    }

    #[test]
    fn test_validate_wrong_version_order() {
        let md = "# Changelog\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- A\n\n## [0.2.0] - 2026-02-01\n\n### Added\n\n- B\n";
        let changelog = Changelog::parse(md).unwrap();
        let issues = changelog.validate();
        assert!(!issues.is_empty());
        assert!(issues[0].contains("descending order"));
    }

    #[test]
    fn test_diff_between_versions() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let diff = changelog.diff("0.1.0", "0.2.0").unwrap();
        // 0.2.0 has "New widget" (Added) and "Crash on startup" (Fixed)
        // 0.1.0 has "Initial release" (Added)
        // All entries in 0.2.0 differ from 0.1.0
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn test_diff_missing_version() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        assert!(changelog.diff("0.1.0", "9.9.9").is_none());
    }

    #[test]
    fn test_get_version() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        assert!(changelog.get_version("0.1.0").is_some());
        assert!(changelog.get_version("0.2.0").is_some());
        assert!(changelog.get_version("9.9.9").is_none());
    }

    #[test]
    fn test_latest_version() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let latest = changelog.latest_version().unwrap();
        assert_eq!(latest.name, "0.2.0");
    }

    #[test]
    fn test_unreleased() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let unreleased = changelog.unreleased().unwrap();
        assert!(unreleased.is_unreleased());
        assert_eq!(unreleased.entries.len(), 1);
    }

    #[test]
    fn test_empty_input() {
        let result = Changelog::parse("");
        assert_eq!(result, Err(ParseError::EmptyInput));
    }

    #[test]
    fn test_entries_by_category() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let v020 = changelog.get_version("0.2.0").unwrap();
        let added = v020.entries_by_category(Category::Added);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].description, "New widget");
        let fixed = v020.entries_by_category(Category::Fixed);
        assert_eq!(fixed.len(), 1);
        let deprecated = v020.entries_by_category(Category::Deprecated);
        assert!(deprecated.is_empty());
    }

    #[test]
    fn test_category_display() {
        assert_eq!(Category::Added.to_string(), "Added");
        assert_eq!(Category::Changed.to_string(), "Changed");
        assert_eq!(Category::Deprecated.to_string(), "Deprecated");
        assert_eq!(Category::Removed.to_string(), "Removed");
        assert_eq!(Category::Fixed.to_string(), "Fixed");
        assert_eq!(Category::Security.to_string(), "Security");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!("Added".parse::<Category>().unwrap(), Category::Added);
        assert_eq!("added".parse::<Category>().unwrap(), Category::Added);
        assert_eq!("FIXED".parse::<Category>().unwrap(), Category::Fixed);
        assert_eq!("Security".parse::<Category>().unwrap(), Category::Security);
        assert!("invalid".parse::<Category>().is_err());
    }

    #[test]
    fn test_multi_line_entries() {
        let md = "\
# Changelog

## [0.1.0] - 2026-03-19

### Added

- This is a long entry that spans
  multiple lines of text
- Single line entry
";
        let changelog = Changelog::parse(md).unwrap();
        let ver = changelog.get_version("0.1.0").unwrap();
        assert_eq!(ver.entries.len(), 2);
        assert_eq!(
            ver.entries[0].description,
            "This is a long entry that spans multiple lines of text"
        );
        assert_eq!(ver.entries[1].description, "Single line entry");
    }

    #[test]
    fn test_parse_error_display() {
        assert_eq!(ParseError::EmptyInput.to_string(), "input is empty");
        assert_eq!(
            ParseError::InvalidFormat("bad".to_string()).to_string(),
            "invalid format: bad"
        );
        assert_eq!(
            ParseError::InvalidCategory("nope".to_string()).to_string(),
            "invalid category: nope"
        );
    }

    #[test]
    fn test_versions_slice() {
        let changelog = Changelog::parse(SAMPLE_CHANGELOG).unwrap();
        let versions = changelog.versions();
        assert_eq!(versions.len(), 3);
    }
}
