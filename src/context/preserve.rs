use std::collections::HashMap;

/// Marker prefix/suffix for generated sections.
const START_MARKER: &str = "<!-- kimono:start:";
const END_MARKER: &str = "<!-- kimono:end:";
const MARKER_CLOSE: &str = " -->";

/// A block of content that lives outside any generated section markers.
/// These blocks represent user-owned content that must be preserved across
/// regeneration cycles.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomBlock {
    /// Where this custom block sits relative to generated sections.
    /// `None` means it's before the first section or after the last.
    pub after_section: Option<String>,
    /// The raw text content.
    pub content: String,
}

/// Extract generated sections from content: returns a map of
/// section_name -> content (including the markers themselves).
pub fn extract_generated_sections(content: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if let Some(name) = parse_start_marker(lines[i]) {
            let mut section_lines = vec![lines[i].to_string()];
            i += 1;
            while i < lines.len() {
                section_lines.push(lines[i].to_string());
                if parse_end_marker(lines[i]).as_deref() == Some(name.as_str()) {
                    break;
                }
                i += 1;
            }
            sections.insert(name, section_lines.join("\n"));
        }
        i += 1;
    }

    sections
}

/// Extract user content that lives OUTSIDE of generated section markers.
/// Returns a list of `CustomBlock` values preserving their position relative
/// to generated sections.
pub fn extract_custom_content(content: &str) -> Vec<CustomBlock> {
    let mut blocks: Vec<CustomBlock> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut last_section: Option<String> = None;
    let mut custom_lines: Vec<String> = Vec::new();

    while i < lines.len() {
        if let Some(name) = parse_start_marker(lines[i]) {
            // Flush accumulated custom lines.
            let text = custom_lines.join("\n");
            if !text.trim().is_empty() {
                blocks.push(CustomBlock {
                    after_section: last_section.clone(),
                    content: text,
                });
            }
            custom_lines.clear();

            // Skip past the end of this section.
            i += 1;
            while i < lines.len() {
                if parse_end_marker(lines[i]).as_deref() == Some(name.as_str()) {
                    last_section = Some(name.clone());
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        custom_lines.push(lines[i].to_string());
        i += 1;
    }

    // Flush any trailing custom content.
    let text = custom_lines.join("\n");
    if !text.trim().is_empty() {
        blocks.push(CustomBlock {
            after_section: last_section,
            content: text,
        });
    }

    blocks
}

/// Merge newly generated content with an existing file, preserving custom
/// blocks from the existing content while replacing generated sections with
/// new versions.
///
/// Algorithm:
/// 1. Parse existing file into generated sections and custom blocks.
/// 2. Parse new generated file into its generated sections.
/// 3. Walk through the existing file structure, replacing generated sections
///    with new versions and keeping custom blocks intact.
/// 4. Append any new sections that didn't exist in the old file.
pub fn merge(generated: &str, existing: &str) -> String {
    let new_sections = extract_generated_sections(generated);
    let old_sections = extract_generated_sections(existing);
    let custom_blocks = extract_custom_content(existing);

    // Track which new sections we've placed.
    let mut placed_sections: Vec<String> = Vec::new();

    // Rebuild: walk through the existing file's structure.
    let mut result_parts: Vec<String> = Vec::new();

    // First, add any custom content before the first section.
    for block in &custom_blocks {
        if block.after_section.is_none() {
            result_parts.push(block.content.clone());
        }
    }

    // Walk through sections in the order they appear in the existing file.
    let section_order = extract_section_order(existing);

    for section_name in &section_order {
        // Use the new version if available, otherwise keep the old one.
        if let Some(new_content) = new_sections.get(section_name) {
            result_parts.push(new_content.clone());
        } else if let Some(old_content) = old_sections.get(section_name) {
            result_parts.push(old_content.clone());
        }
        placed_sections.push(section_name.clone());

        // Add any custom blocks that follow this section.
        for block in &custom_blocks {
            if block.after_section.as_deref() == Some(section_name) {
                result_parts.push(block.content.clone());
            }
        }
    }

    // Append new sections that didn't exist in the old file.
    for (name, content) in &new_sections {
        if !placed_sections.contains(name) {
            result_parts.push(content.clone());
        }
    }

    result_parts.join("\n")
}

/// Extract the order of sections as they appear in the content.
fn extract_section_order(content: &str) -> Vec<String> {
    let mut order = Vec::new();
    for line in content.lines() {
        if let Some(name) = parse_start_marker(line) {
            order.push(name);
        }
    }
    order
}

/// Try to parse a start marker from a line.
/// Returns `Some(section_name)` if the line is `<!-- kimono:start:NAME -->`.
fn parse_start_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with(START_MARKER) && trimmed.ends_with(MARKER_CLOSE) {
        let inner = &trimmed[START_MARKER.len()..trimmed.len() - MARKER_CLOSE.len()];
        if !inner.is_empty() {
            return Some(inner.to_string());
        }
    }
    None
}

/// Try to parse an end marker from a line.
/// Returns `Some(section_name)` if the line is `<!-- kimono:end:NAME -->`.
fn parse_end_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with(END_MARKER) && trimmed.ends_with(MARKER_CLOSE) {
        let inner = &trimmed[END_MARKER.len()..trimmed.len() - MARKER_CLOSE.len()];
        if !inner.is_empty() {
            return Some(inner.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_generated() -> String {
        r#"# My Workspace

<!-- kimono:start:overview -->
## Overview
This is a workspace with 2 repos.
<!-- kimono:end:overview -->

<!-- kimono:start:registry -->
## Registry
| Repo | Description |
|------|-------------|
| backend | API server |
| frontend | Web app |
<!-- kimono:end:registry -->

<!-- kimono:start:commands -->
## Commands
Various commands here.
<!-- kimono:end:commands -->"#
            .to_string()
    }

    fn sample_existing_with_custom() -> String {
        r#"# My Workspace

My custom intro text here.

<!-- kimono:start:overview -->
## Overview
OLD overview content.
<!-- kimono:end:overview -->

Custom note between overview and registry.

<!-- kimono:start:registry -->
## Registry
OLD registry content.
<!-- kimono:end:registry -->

<!-- kimono:start:commands -->
## Commands
OLD commands.
<!-- kimono:end:commands -->

My custom footer here."#
            .to_string()
    }

    #[test]
    fn test_extract_generated_sections() {
        let content = sample_generated();
        let sections = extract_generated_sections(&content);

        assert_eq!(sections.len(), 3);
        assert!(sections.contains_key("overview"));
        assert!(sections.contains_key("registry"));
        assert!(sections.contains_key("commands"));

        let overview = &sections["overview"];
        assert!(overview.contains("kimono:start:overview"));
        assert!(overview.contains("kimono:end:overview"));
        assert!(overview.contains("This is a workspace with 2 repos."));
    }

    #[test]
    fn test_extract_custom_content() {
        let content = sample_existing_with_custom();
        let blocks = extract_custom_content(&content);

        // Should have: before-first, between-overview-and-registry, after-commands
        assert_eq!(
            blocks.len(),
            3,
            "expected 3 custom blocks, got: {:?}",
            blocks
        );

        // Before first section
        assert!(blocks[0].after_section.is_none());
        assert!(blocks[0].content.contains("My custom intro text here."));

        // Between overview and registry
        assert_eq!(blocks[1].after_section.as_deref(), Some("overview"));
        assert!(blocks[1]
            .content
            .contains("Custom note between overview and registry."));

        // After commands (footer)
        assert_eq!(blocks[2].after_section.as_deref(), Some("commands"));
        assert!(blocks[2].content.contains("My custom footer here."));
    }

    #[test]
    fn test_extract_and_merge_round_trip() {
        let existing = sample_existing_with_custom();
        let new_generated = sample_generated();

        let merged = merge(&new_generated, &existing);

        // Custom content should be preserved.
        assert!(
            merged.contains("My custom intro text here."),
            "intro custom text should be preserved"
        );
        assert!(
            merged.contains("Custom note between overview and registry."),
            "mid-section custom text should be preserved"
        );
        assert!(
            merged.contains("My custom footer here."),
            "footer custom text should be preserved"
        );

        // Generated sections should be updated to new versions.
        assert!(
            merged.contains("This is a workspace with 2 repos."),
            "new overview content should be present"
        );
        assert!(
            !merged.contains("OLD overview content."),
            "old overview content should be replaced"
        );
        assert!(
            merged.contains("| backend | API server |"),
            "new registry content should be present"
        );
        assert!(
            !merged.contains("OLD registry content."),
            "old registry content should be replaced"
        );
    }

    #[test]
    fn test_preserve_content_before_first_marker() {
        let existing = r#"Some user text at the top.

<!-- kimono:start:overview -->
Old overview.
<!-- kimono:end:overview -->"#;

        let generated = r#"<!-- kimono:start:overview -->
New overview.
<!-- kimono:end:overview -->"#;

        let merged = merge(generated, existing);

        assert!(
            merged.contains("Some user text at the top."),
            "content before first marker should be preserved"
        );
        assert!(
            merged.contains("New overview."),
            "generated section should be updated"
        );
        assert!(
            !merged.contains("Old overview."),
            "old generated content should be replaced"
        );
    }

    #[test]
    fn test_preserve_content_after_last_marker() {
        let existing = r#"<!-- kimono:start:overview -->
Old overview.
<!-- kimono:end:overview -->

User notes at the bottom.
These should survive."#;

        let generated = r#"<!-- kimono:start:overview -->
New overview.
<!-- kimono:end:overview -->"#;

        let merged = merge(generated, existing);

        assert!(
            merged.contains("User notes at the bottom."),
            "content after last marker should be preserved"
        );
        assert!(
            merged.contains("These should survive."),
            "multi-line trailing content should be preserved"
        );
        assert!(
            merged.contains("New overview."),
            "generated section should be updated"
        );
    }

    #[test]
    fn test_new_section_appended() {
        let existing = r#"<!-- kimono:start:overview -->
Overview content.
<!-- kimono:end:overview -->"#;

        let generated = r#"<!-- kimono:start:overview -->
Updated overview.
<!-- kimono:end:overview -->

<!-- kimono:start:registry -->
New registry section.
<!-- kimono:end:registry -->"#;

        let merged = merge(generated, existing);

        assert!(
            merged.contains("Updated overview."),
            "existing section should be updated"
        );
        assert!(
            merged.contains("New registry section."),
            "new section should be appended"
        );
        assert!(
            merged.contains("kimono:start:registry"),
            "new section markers should be present"
        );
    }

    #[test]
    fn test_parse_start_marker() {
        assert_eq!(
            parse_start_marker("<!-- kimono:start:overview -->"),
            Some("overview".to_string())
        );
        assert_eq!(
            parse_start_marker("  <!-- kimono:start:registry -->  "),
            Some("registry".to_string())
        );
        assert_eq!(parse_start_marker("not a marker"), None);
        assert_eq!(parse_start_marker("<!-- kimono:end:overview -->"), None);
    }

    #[test]
    fn test_parse_end_marker() {
        assert_eq!(
            parse_end_marker("<!-- kimono:end:overview -->"),
            Some("overview".to_string())
        );
        assert_eq!(parse_end_marker("<!-- kimono:start:overview -->"), None);
        assert_eq!(parse_end_marker("plain text"), None);
    }

    #[test]
    fn test_merge_no_existing() {
        let generated = sample_generated();
        // When existing is empty, merge should just return the generated content
        // (possibly with an empty custom block prefix).
        let merged = merge(&generated, "");

        assert!(merged.contains("kimono:start:overview"));
        assert!(merged.contains("kimono:start:registry"));
        assert!(merged.contains("kimono:start:commands"));
    }

    #[test]
    fn test_section_order_preserved() {
        let existing = r#"<!-- kimono:start:commands -->
Commands first.
<!-- kimono:end:commands -->

<!-- kimono:start:overview -->
Overview second.
<!-- kimono:end:overview -->"#;

        let generated = r#"<!-- kimono:start:overview -->
New overview.
<!-- kimono:end:overview -->

<!-- kimono:start:commands -->
New commands.
<!-- kimono:end:commands -->"#;

        let merged = merge(generated, existing);

        // The order in the merged output should follow the existing file's order.
        let commands_pos = merged.find("kimono:start:commands").unwrap();
        let overview_pos = merged.find("kimono:start:overview").unwrap();
        assert!(
            commands_pos < overview_pos,
            "existing section order should be preserved"
        );
    }
}
