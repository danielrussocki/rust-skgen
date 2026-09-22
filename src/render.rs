//! Deterministic skill rendering boundary.

use crate::domain::DocumentationSource;

/// Marks a component that renders a skill from normalized content.
pub trait SkillRenderer {}

/// Renders normalized documentation as a guide with attributable source references.
pub fn render_guide_with_references<S: Clone + Into<DocumentationSource>>(pages: &[S]) -> String {
    let mut pages = pages.iter().cloned().map(Into::into).collect::<Vec<_>>();
    pages.sort_by_cached_key(canonical_source_url);

    let objective = pages.iter().find_map(|page| page.content()).unwrap_or("");
    let instructions = pages
        .iter()
        .filter_map(|page| page.content())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut rendered = format!(
        "# Documentation Guide\n\n## Objective\n\n{objective}\n\n## Instructions\n\n{instructions}\n"
    );

    for page in &pages {
        match page.content() {
            Some(content) => rendered.push_str(&format!(
                "\n### Source: {}\n\n{content}\n",
                page.source_url(),
            )),
            None => rendered.push_str(&unavailable_source_notice("###", page.source_url())),
        }
    }

    rendered.push_str("\n## References\n");
    for page in pages {
        rendered.push_str(&format!("\n- {}", page.source_url()));
    }
    rendered.push('\n');
    rendered
}

/// Renders normalized documentation organized by attributable source page.
pub fn render_organized_content<S: Clone + Into<DocumentationSource>>(pages: &[S]) -> String {
    let mut pages = pages.iter().cloned().map(Into::into).collect::<Vec<_>>();
    pages.sort_by_cached_key(canonical_source_url);

    let mut rendered = String::from("# Organized Documentation\n");
    for page in pages {
        match page.content() {
            Some(content) => rendered.push_str(&format!(
                "\n## Source: {}\n\n{content}\n",
                page.source_url(),
            )),
            None => rendered.push_str(&unavailable_source_notice("##", page.source_url())),
        }
    }
    rendered
}

fn canonical_source_url(page: &DocumentationSource) -> String {
    let mut source_url = page.source_url().clone();
    source_url.set_query(None);
    source_url.set_fragment(None);
    source_url.to_string()
}

fn unavailable_source_notice(heading_level: &str, source_url: &url::Url) -> String {
    format!(
        "\n{heading_level} Unavailable source: {source_url}\n\nDocumentation is unavailable for this source. Search the internet or the source code for current documentation.\n"
    )
}
