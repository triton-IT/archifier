//! Rendu Markdown natif : CommonMark + tableaux, HTML filtré, liens `arch://`
//! réécrits vers des routes applicatives.

use crate::uri::ArchUri;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use std::fmt::Write as _;

/// Rendu HTML sûr d'un document Markdown.
pub fn render(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let events = parser.map(|event| match event {
        // Le HTML brut n'est jamais exécuté : il est rendu comme du texte, que
        // `push_html` échappe lui-même.
        Event::Html(html) => Event::Text(html),
        Event::InlineHtml(html) => Event::Text(html),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: CowStr::from(rewrite_href(&dest_url)),
            title,
            id,
        }),
        other => other,
    });

    let mut out = String::new();
    html::push_html(&mut out, events);
    out
}

fn rewrite_href(dest: &str) -> String {
    if let Ok(uri) = dest.parse::<ArchUri>() {
        return uri.route();
    }
    let lowered = dest.trim().to_ascii_lowercase();
    if lowered.starts_with("javascript:") || lowered.starts_with("data:text/html") {
        return "#".to_string();
    }
    dest.to_string()
}
/// Texte brut d'un document Markdown, pour l'indexation plein texte.
pub fn plain_text(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let mut out = String::new();
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(t) | Event::Code(t) => {
                out.push_str(&t);
                out.push(' ');
            }
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => out.push('\n'),
            _ => {}
        }
    }
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out
}

/// Extrait un résumé textuel court, sans balisage.
pub fn excerpt(markdown: &str, max_chars: usize) -> String {
    let text = plain_text(markdown);
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out = String::new();
    for word in trimmed.split_whitespace() {
        if out.chars().count() + word.chars().count() + 1 > max_chars {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    let _ = write!(out, "…");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    #[test]
    fn rewrites_internal_links() {
        let html = render(&format!("Voir [Customer API](arch://interface/{ID})."));
        assert!(html.contains(&format!("#/view/interfaces/{ID}")), "{html}");
    }

    #[test]
    fn escapes_raw_html() {
        let html = render("<script>alert('x')</script>\n\nBonjour");
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn neutralises_javascript_urls() {
        let html = render("[clic](javascript:alert(1))");
        assert!(!html.contains("javascript:alert"), "{html}");
    }

    #[test]
    fn renders_tables() {
        let html = render("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"), "{html}");
    }

    #[test]
    fn plain_text_strips_markup() {
        let text = plain_text("# Titre\n\nLe **contrôle** de `solvabilité`.");
        assert!(text.contains("contrôle"));
        assert!(!text.contains('#'));
        assert!(!text.contains('*'));
    }
}
