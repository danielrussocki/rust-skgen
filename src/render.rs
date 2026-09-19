//! Deterministic skill rendering boundary.

use crate::domain::DocumentationPage;

/// Marks a component that renders a skill from normalized content.
pub trait SkillRenderer {}

/// Renders normalized documentation as a guide with attributable source references.
pub fn render_guide_with_references(pages: &[DocumentationPage]) -> String {
    let mut pages = pages.iter().collect::<Vec<_>>();
    pages.sort_by_cached_key(|page| canonical_source_url(page));

    let objective = pages.first().map_or("", |page| page.content());
    let mut rendered =
        format!("# Documentation Guide\n\n## Objective\n\n{objective}\n\n## Instructions\n");

    for page in &pages {
        rendered.push_str(&format!(
            "\n### Source: {}\n\n{}\n",
            page.source_url(),
            page.content()
        ));
    }

    rendered.push_str("\n## References\n");
    for page in pages {
        rendered.push_str(&format!("\n- {}", page.source_url()));
    }
    rendered.push('\n');
    rendered
}

fn canonical_source_url(page: &DocumentationPage) -> String {
    let mut source_url = page.source_url().clone();
    source_url.set_query(None);
    source_url.set_fragment(None);
    source_url.to_string()
}
