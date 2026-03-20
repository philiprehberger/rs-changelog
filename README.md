# rs-changelog

Programmatic CHANGELOG.md parsing, generation, and manipulation following Keep a Changelog format.

## Installation

```toml
[dependencies]
philiprehberger-changelog = "0.1"
```

## Usage

```rust
use philiprehberger_changelog::{Changelog, Category};

// Parse an existing changelog
let markdown = std::fs::read_to_string("CHANGELOG.md")?;
let mut changelog = Changelog::parse(&markdown)?;

// Add an entry
changelog.add_entry("Unreleased", Category::Fixed, "Resolved panic on empty input");

// Release a version
changelog.release("0.2.0", "2026-03-19");

// Render back to markdown
let output = changelog.to_markdown();
std::fs::write("CHANGELOG.md", output)?;
```

### Validation

```rust
let issues = changelog.validate();
if issues.is_empty() {
    println!("Changelog is valid!");
} else {
    for issue in &issues {
        println!("Issue: {}", issue);
    }
}
```

### Diff between versions

```rust
if let Some(changes) = changelog.diff("0.1.0", "0.2.0") {
    for entry in &changes {
        println!("[{}] {}", entry.category, entry.description);
    }
}
```

## API

| Function / Type | Description |
|----------------|-------------|
| `Changelog::parse(md)` | Parse Keep a Changelog markdown |
| `.add_entry(version, category, desc)` | Add an entry to a version |
| `.release(version, date)` | Promote Unreleased to named version |
| `.get_version(name)` | Find a version by name |
| `.latest_version()` | Get the most recent release |
| `.unreleased()` | Get the Unreleased section |
| `.to_markdown()` | Render back to markdown |
| `.validate()` | Check format compliance |
| `.diff(v1, v2)` | Get entries added between versions |

## License

MIT
