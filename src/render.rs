//! HTML rendering module.
//!
//! This module is responsible for generating the raw HTML returned to the client.
//! It takes domain models and query results, and formats them into UI components.

use crate::{
    data::Catalog,
    domain::{Card, Color, SortMode},
    query::{CardQuery, Direction, SearchOptions, ViewMode},
};
use html_escape::{encode_double_quoted_attribute, encode_safe};
use rand::seq::SliceRandom;
use std::collections::{BTreeMap, HashMap, HashSet};
use url::form_urlencoded;

/// The embedded CSS stylesheet for the application.
pub const CSS: &str = include_str!("../assets/site.css");

pub enum Rendered {
    Html(String),
    Redirect(String),
}

pub fn home(catalog: &Catalog) -> String {
    let body = format!(
        r#"
<section class="hero">
  <div class="hero-copy">
    <p class="eyebrow">Slay the Spire card search</p>
    <h1>Search cards by text, tags, class, cost, and shape.</h1>
    <p>Browse {} cards and {} tags from the local Spire Tagger database.</p>
  </div>
  {}
  <div class="quick-actions">
    <a href="/advanced">Advanced search</a>
    <a href="/syntax">Syntax guide</a>
    <a href="/tags">Tag database</a>
    <a href="/tag-graph">Tag graph</a>
    <a href="/random">Random card</a>
  </div>
</section>
<section class="band">
  <h2>Fast Starts</h2>
  <div class="query-chips">
    {}
    {}
    {}
    {}
    {}
  </div>
</section>"#,
        catalog.cards().len(),
        catalog.tags().len(),
        search_form("", false),
        query_chip("tag:damage color:red"),
        query_chip("tag:block type:skill"),
        query_chip("tag:fatten"),
        query_chip("rarity:rare -tag:damage"),
        query_chip("cost:0 tag:card-draw"),
    );
    page("Search", &body, "home")
}

pub fn results(catalog: &Catalog, params: &HashMap<String, String>) -> String {
    let options = SearchOptions::from_params(params);
    let cards = options.filter(catalog.cards());
    let total_pages = cards.len().div_ceil(options.per_page).max(1);
    let page_num = options.page.min(total_pages);
    let start = (page_num - 1) * options.per_page;
    let visible = &cards[start..cards.len().min(start + options.per_page)];

    let results = match options.view {
        ViewMode::Text => text_results(visible),
        ViewMode::Checklist => checklist_results(visible, &options, params),
        ViewMode::Images => image_results(visible),
    };

    let body = format!(
        r#"
<section class="results-head">
  {}
  {}
</section>
{}
{}"#,
        search_form(&options.query_text, true),
        result_controls(params, &options, cards.len(), page_num, total_pages),
        results,
        pagination(params, page_num, total_pages),
    );
    page(
        &format!(
            "Results for {}",
            if options.query_text.is_empty() {
                "all cards"
            } else {
                &options.query_text
            }
        ),
        &body,
        "home",
    )
}

pub fn card_page(catalog: &Catalog, name: &str) -> Option<String> {
    let cards = catalog.cards_in_display_order();
    let index = cards
        .iter()
        .position(|card| card.name.eq_ignore_ascii_case(name))?;
    let card = cards[index];
    let previous = cards[(index + cards.len() - 1) % cards.len()];
    let next = cards[(index + 1) % cards.len()];
    let wiki_url = format!(
        "https://slay-the-spire.fandom.com/wiki/{}",
        path_segment(&card.name.replace(' ', "_"))
    );
    let body = format!(
        r#"
<section class="card-page">
  <div>{}</div>
  <article class="detail-panel">
    <div class="card-nav">
      <a href="/card/{}">Previous</a>
      <a href="/card/{}">Next</a>
    </div>
    <h1>{}</h1>
    <p>{}</p>
    <div class="badges">{} {} {} {}</div>
    <h2>Tags</h2>
    {}
    <h2>Card Metrics</h2>
    <dl class="metrics">
      <div><dt>Raw tags</dt><dd>{}</dd></div>
      <div><dt>Inherited tags</dt><dd>{}</dd></div>
      <div><dt>Upgrade value</dt><dd>Placeholder</dd></div>
      <div><dt>Generated cards</dt><dd>Placeholder</dd></div>
    </dl>
    <div class="actions">
      <a href="{}">Wiki page</a>
      <a href="/tags">Suggest or dispute a tag</a>
    </div>
  </article>
</section>"#,
        card_art(card),
        path_segment(&previous.name),
        path_segment(&next.name),
        text(&card.name),
        text(&card.body),
        badge(&card.color.to_string()),
        badge(&card.kind.to_string()),
        badge(&card.rarity.to_string()),
        badge(&cost_label(card)),
        tag_tree(card),
        card.tags.len(),
        card.inherited_tags.len(),
        attr(&wiki_url),
    );
    Some(page(&card.name, &body, "home"))
}

pub fn tags_page(catalog: &Catalog) -> String {
    let rows = catalog
        .tags()
        .iter()
        .map(|tag| {
            let query = CardQuery::parse(&format!("tag:{tag}"));
            let count = catalog
                .cards()
                .iter()
                .filter(|card| query.matches(card))
                .count();
            let parents = catalog.tag_graph().parents(tag).join(", ");
            format!(
                r#"<tr><td><a href="/tag/{}">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                path_segment(tag),
                text(tag),
                count,
                text(&parents)
            )
        })
        .collect::<String>();
    let body = format!(
        r#"
<section class="page-head">
  <h1>Tag Database</h1>
  <p>Direct database tags plus inherited tags from the local tagger rules.</p>
  <div class="quick-actions">
    <a href="/tag-graph">View graph</a>
  </div>
</section>
<section class="table-wrap">
  <table><thead><tr><th>Tag</th><th>Cards</th><th>Parents</th></tr></thead><tbody>{rows}</tbody></table>
</section>"#
    );
    page("Tags", &body, "tags")
}

pub fn tag_graph_page(catalog: &Catalog) -> String {
    let graph = catalog.tag_graph();
    let inheritance_edges = graph.edges();
    let card_tag_edge_count = catalog
        .cards()
        .iter()
        .map(|card| card.tags.len())
        .sum::<usize>();
    let tag_stats = tag_graph_stats(catalog);
    let tag_depths = tag_graph_depths(catalog, &inheritance_edges);
    let svg = tag_graph_svg(catalog, &inheritance_edges, &tag_stats, &tag_depths);
    let top_tag_rows = top_tag_rows(&tag_stats);
    let edge_rows = inheritance_edges
        .iter()
        .map(|edge| {
            format!(
                r#"<tr><td><a href="/tag/{}">{}</a></td><td><a href="/tag/{}">{}</a></td></tr>"#,
                path_segment(&edge.child),
                text(&edge.child),
                path_segment(&edge.parent),
                text(&edge.parent)
            )
        })
        .collect::<String>();
    let body = format!(
        r#"
<section class="page-head">
  <h1>Tag Graph</h1>
  <p>{} cards and {} tags connected by {} card-tag edges and {} inheritance edges. Cards feed into base tags; tags then flow into inherited parent tags moving rightward through the tree.</p>
</section>
<section class="graph-panel">
  {}
</section>
<section class="graph-summary">
  <h2>Largest Tags</h2>
  <div class="table-wrap">
    <table><thead><tr><th>Tag</th><th>Cards</th><th>Color mix</th></tr></thead><tbody>{}</tbody></table>
  </div>
  <h2>Inheritance Edges</h2>
  <div class="table-wrap">
    <table><thead><tr><th>Child tag</th><th>Inherited parent</th></tr></thead><tbody>{}</tbody></table>
  </div>
</section>"#,
        catalog.cards().len(),
        graph.node_count(),
        card_tag_edge_count,
        inheritance_edges.len(),
        svg,
        top_tag_rows,
        edge_rows
    );
    page("Tag Graph", &body, "graph")
}

pub fn tag_page(catalog: &Catalog, tag: &str, params: &HashMap<String, String>) -> String {
    let query = CardQuery::parse(&format!("tag:{tag}"));
    let matching: Vec<_> = catalog
        .cards()
        .iter()
        .filter(|card| query.matches(card))
        .collect();
    let threshold = params
        .get("threshold")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(15)
        .clamp(1, 100);
    let total = matching.len().max(1);
    let counts = Color::all_displayed()
        .into_iter()
        .map(|color| {
            (
                color,
                matching.iter().filter(|card| card.color == color).count(),
            )
        })
        .collect::<Vec<_>>();
    let distribution = counts
        .iter()
        .map(|(color, count)| {
            format!(
                r#"<div><span>{}</span><meter min="0" max="{}" value="{}"></meter><strong>{}</strong></div>"#,
                color, total, count, count
            )
        })
        .collect::<String>();
    let color_effect = counts
        .iter()
        .filter_map(|(color, count)| {
            ((*count * 100 / total) >= threshold && *count > 0).then_some(color.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ");
    let children = catalog.tag_graph().children_of(tag);
    let body = format!(
        r#"
<section class="page-head">
  <h1>{}</h1>
  <p>{} matching cards. Color-effect threshold: {}%.</p>
  <form class="threshold-form" action="/tag/{}" method="get">
    <label>Threshold <input type="number" min="1" max="100" name="threshold" value="{}"></label>
    <button type="submit">Update</button>
  </form>
</section>
<section class="tag-detail-grid">
  <article>
    <h2>Relationships</h2>
    <h3>Parents</h3>
    <p>{}</p>
    <h3>Children</h3>
    <p>{}</p>
  </article>
  <article>
    <h2>Color Distribution</h2>
    <div class="distribution">{}</div>
    <p class="muted">Current color-effect read: {}</p>
  </article>
</section>
<section>
  <h2>Cards</h2>
  {}
</section>"#,
        text(tag),
        matching.len(),
        threshold,
        path_segment(tag),
        threshold,
        tag_links(catalog.tag_graph().parents(tag).iter().map(String::as_str)),
        tag_links(children),
        distribution,
        text(if color_effect.is_empty() {
            "none"
        } else {
            &color_effect
        }),
        image_results(&matching),
    );
    page(tag, &body, "tags")
}

pub fn syntax_page() -> String {
    let examples = [
        ("Plain text", "bash", "Finds words in card names only."),
        (
            "Exact-ish phrase",
            "\"gain block\"",
            "Quote a phrase when it includes spaces.",
        ),
        ("Card name", "name:bash or n:bash", "Search card names."),
        (
            "Rules text",
            "body:vulnerable or o:vulnerable",
            "Search card descriptions.",
        ),
        ("Tags", "tag:damage -tag:fatten", "Require or exclude tags."),
        (
            "Color",
            "color:red or c:blue",
            "Use red, green, blue, purple, or colorless.",
        ),
        (
            "Type",
            "type:attack",
            "Use attack, skill, power, status, or curse.",
        ),
        (
            "Rarity",
            "rarity:rare",
            "Use basic, common, uncommon, rare, or special.",
        ),
        (
            "Cost",
            "cost:0, cost:<=1, cost:>2",
            "Compare leading numeric energy costs.",
        ),
    ];
    let rows = examples
        .iter()
        .map(|(goal, query, notes)| {
            format!(
                "<tr><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                text(goal),
                text(query),
                text(notes)
            )
        })
        .collect::<String>();
    let body = format!(
        r#"
<section class="page-head">
  <h1>Search Syntax</h1>
  <p>A compact Scryfall-inspired query language for the local card/tag database.</p>
</section>
<section class="table-wrap syntax-table">
  <table><thead><tr><th>Goal</th><th>Query</th><th>Notes</th></tr></thead><tbody>{rows}</tbody></table>
</section>"#
    );
    page("Syntax", &body, "syntax")
}

pub fn advanced_page(params: &HashMap<String, String>) -> String {
    let q = params.get("q").map(String::as_str).unwrap_or("");
    let body = format!(
        r#"
<section class="page-head">
  <h1>Advanced Search</h1>
  <p>Use fields to generate the same query syntax the main search accepts.</p>
</section>
<form class="advanced-form" action="/search" method="get">
  <label>Base query <input name="q" value="{}"></label>
  <label>Name <input name="name" placeholder="bash"></label>
  <label>Rules text <input name="body" placeholder="vulnerable"></label>
  <label>Tag <input name="tag" placeholder="damage"></label>
  <label>Color
    <select name="color"><option value="">Any</option><option>red</option><option>green</option><option>blue</option><option>purple</option><option>colorless</option></select>
  </label>
  <label>Type
    <select name="type"><option value="">Any</option><option>attack</option><option>skill</option><option>power</option><option>status</option><option>curse</option></select>
  </label>
  <label>Rarity
    <select name="rarity"><option value="">Any</option><option>basic</option><option>common</option><option>uncommon</option><option>rare</option><option>special</option></select>
  </label>
  <label>Cost <input name="cost" placeholder="0, <=1, >2"></label>
  <label>Checklist columns <input name="columns" value="damage,block,card-draw"></label>
  <label>View
    <select name="view"><option value="images">Images</option><option value="text">Text</option><option value="checklist">Checklist</option></select>
  </label>
  <button type="submit">Search Cards</button>
</form>"#,
        attr(q)
    );
    page("Advanced Search", &body, "advanced")
}

pub fn random_card(catalog: &Catalog, params: &HashMap<String, String>) -> Option<Rendered> {
    let options = SearchOptions::from_params(params);
    let cards = options.filter(catalog.cards());
    let card = cards.choose(&mut rand::thread_rng())?;
    Some(Rendered::Redirect(format!(
        "/card/{}",
        path_segment(&card.name)
    )))
}

pub fn error_page(status: u16, message: &str) -> String {
    let body = format!(
        r#"<section class="empty"><h1>{}</h1><p>{}</p><a href="/">Return to search</a></section>"#,
        status,
        text(message)
    );
    page("Error", &body, "")
}

fn page(title: &str, body: &str, active: &str) -> String {
    let nav = [
        ("Search", "/", "home"),
        ("Advanced", "/advanced", "advanced"),
        ("Syntax", "/syntax", "syntax"),
        ("Tags", "/tags", "tags"),
        ("Graph", "/tag-graph", "graph"),
        ("Random", "/random", "random"),
    ]
    .iter()
    .map(|(label, href, key)| {
        format!(
            r#"<a class="{}" href="{}">{}</a>"#,
            if *key == active { "active" } else { "" },
            href,
            label
        )
    })
    .collect::<String>();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{} - Spire Tagger</title>
  <link rel="stylesheet" href="/static/site.css">
</head>
<body>
  <header class="site-header">
    <a class="brand" href="/">Spire Tagger</a>
    <nav>{}</nav>
  </header>
  <main>{}</main>
</body>
</html>"#,
        text(title),
        nav,
        body
    )
}

fn search_form(query: &str, compact: bool) -> String {
    format!(
        r#"
<form class="search-form {}" action="/search" method="get">
  <div class="search-row">
    <input name="q" value="{}" placeholder='Try bash, body:"gain block", tag:damage color:red, -tag:fatten' autofocus>
    <button type="submit">Search</button>
  </div>
  <div class="filters">
    <label>Database
      <select name="game">
        <option value="sts1">Slay the Spire 1</option>
        <option value="sts2" disabled>Slay the Spire 2 placeholder</option>
        <option value="sts2-beta" disabled>STS 2 beta placeholder</option>
      </select>
    </label>
    <label>Mods
      <select name="mods">
        <option value="">Base game</option>
        <option disabled>Supported mods placeholder</option>
      </select>
    </label>
    <label class="checkbox"><input type="checkbox" name="multiplayer"> Include multiplayer</label>
  </div>
</form>"#,
        if compact { "compact" } else { "" },
        attr(query)
    )
}

fn result_controls(
    params: &HashMap<String, String>,
    options: &SearchOptions,
    count: usize,
    page_num: usize,
    total_pages: usize,
) -> String {
    let sort_options = SortMode::all()
        .iter()
        .map(|mode| {
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                mode.key(),
                selected(*mode == options.sort),
                mode.label()
            )
        })
        .collect::<String>();
    format!(
        r#"
<div class="toolbar">
  <div><strong>{}</strong> cards found <span>Page {} of {}</span></div>
  <div class="segmented">
    {}
    {}
    {}
  </div>
  <form action="/search" method="get" class="sort-form">
    {}
    <label>Sort <select name="sort">{}</select></label>
    <label>Direction
      <select name="dir">
        <option value="asc" {}>Ascending</option>
        <option value="desc" {}>Descending</option>
      </select>
    </label>
    <button type="submit">Apply</button>
  </form>
  <a class="button-secondary" href="/random?{}">Random in search</a>
</div>"#,
        count,
        page_num,
        total_pages,
        view_link(params, options.view, ViewMode::Images, "Images"),
        view_link(params, options.view, ViewMode::Text, "Text"),
        view_link(params, options.view, ViewMode::Checklist, "Checklist"),
        hidden_inputs(params, &["sort", "dir", "page"]),
        sort_options,
        selected(options.direction == Direction::Asc),
        selected(options.direction == Direction::Desc),
        query_string(&[("q", options.query_text.as_str())]),
    )
}

fn image_results(cards: &[&Card]) -> String {
    if cards.is_empty() {
        return r#"<section class="empty">No cards matched that search.</section>"#.to_string();
    }
    format!(
        r#"<section class="card-grid">{}</section>"#,
        cards
            .iter()
            .map(|card| format!(
                r#"<a class="card-link" href="/card/{}">{}</a>"#,
                path_segment(&card.name),
                card_art(card)
            ))
            .collect::<String>()
    )
}

fn text_results(cards: &[&Card]) -> String {
    if cards.is_empty() {
        return r#"<section class="empty">No cards matched that search.</section>"#.to_string();
    }
    format!(
        r#"<section class="text-results">{}</section>"#,
        cards
            .iter()
            .map(|card| {
                let tag_sample = card.all_tags().take(8).collect::<Vec<_>>().join(", ");
                format!(
                    r#"
<article class="text-result">
  <h2><a href="/card/{}">{}</a></h2>
  <p>{}</p>
  <div>{} {} {} {}</div>
  <p class="muted">{}</p>
</article>"#,
                    path_segment(&card.name),
                    text(&card.name),
                    text(&card.body),
                    badge(&card.color.to_string()),
                    badge(&card.kind.to_string()),
                    badge(&card.rarity.to_string()),
                    badge(&cost_label(card)),
                    text(&tag_sample)
                )
            })
            .collect::<String>()
    )
}

fn checklist_results(
    cards: &[&Card],
    options: &SearchOptions,
    params: &HashMap<String, String>,
) -> String {
    if cards.is_empty() {
        return r#"<section class="empty">No cards matched that search.</section>"#.to_string();
    }
    let header = options
        .columns
        .iter()
        .map(|column| format!("<th>{}</th>", text(column)))
        .collect::<String>();
    let rows = cards
        .iter()
        .map(|card| {
            let cells = options
                .columns
                .iter()
                .map(|column| {
                    let query = CardQuery::parse(&format!("tag:{column}"));
                    format!(
                        r#"<td class="bool">{}</td>"#,
                        if query.matches(card) { "Y" } else { "" }
                    )
                })
                .collect::<String>();
            format!(
                r#"
<tr>
  <td><a href="/card/{}">{}</a></td>
  <td>{}</td>
  <td>{}</td>
  <td>{}</td>
  <td>{}</td>
  {}
</tr>"#,
                path_segment(&card.name),
                text(&card.name),
                card.color,
                card.kind,
                card.rarity,
                text(&card.cost.to_string()),
                cells
            )
        })
        .collect::<String>();
    format!(
        r#"
<section class="checklist">
  <form action="/search" method="get" class="column-form">
    {}
    <label>Tag columns <input name="columns" value="{}"></label>
    <button type="submit">Update</button>
  </form>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>Color</th><th>Type</th><th>Rarity</th><th>Cost</th>{}</tr></thead>
      <tbody>{}</tbody>
    </table>
  </div>
</section>"#,
        hidden_inputs(params, &["columns", "page"]),
        attr(&options.columns.join(",")),
        header,
        rows
    )
}

fn pagination(params: &HashMap<String, String>, page_num: usize, total_pages: usize) -> String {
    if total_pages <= 1 {
        return String::new();
    }
    let links = [
        ("First", 1),
        ("Previous", page_num.saturating_sub(1).max(1)),
        ("Next", (page_num + 1).min(total_pages)),
        ("Last", total_pages),
    ]
    .iter()
    .map(|(label, target)| {
        format!(
            r#"<a href="{}">{}</a>"#,
            attr(&updated_query(params, &[("page", &target.to_string())])),
            label
        )
    })
    .collect::<String>();
    format!(
        r#"
<nav class="pagination">
  {}
  <form action="/search" method="get">
    {}
    <label>Page <input name="page" type="number" min="1" max="{}" value="{}"></label>
    <button type="submit">Go</button>
  </form>
</nav>"#,
        links,
        hidden_inputs(params, &["page"]),
        total_pages,
        page_num
    )
}

fn card_art(card: &Card) -> String {
    format!(
        r#"
<div class="card-art card-{}">
  <div class="card-top">
    <strong>{}</strong>
    <span>{}</span>
  </div>
  <div class="card-image-placeholder"></div>
  <div class="card-text">
    <p>{}</p>
  </div>
  <div class="card-bottom">
    <span>{}</span>
    <span>{}</span>
  </div>
</div>"#,
        card.color,
        text(&card.name),
        text(&card.cost.to_string()),
        text(&card.body),
        card.kind,
        card.rarity
    )
}

fn tag_tree(card: &Card) -> String {
    let raw = tag_list(&card.tags);
    let inherited = tag_list(&card.inherited_tags);
    format!(
        r#"
<div class="tag-columns">
  <div><h3>Direct</h3><ul class="tag-list">{}</ul></div>
  <div><h3>Inherited</h3><ul class="tag-list inherited">{}</ul></div>
</div>"#,
        raw, inherited
    )
}

fn tag_list(tags: &[String]) -> String {
    if tags.is_empty() {
        return "<li>None</li>".to_string();
    }
    tags.iter()
        .map(|tag| {
            format!(
                r#"<li><a href="/tag/{}">{}</a></li>"#,
                path_segment(tag),
                text(tag)
            )
        })
        .collect()
}

fn tag_links<'a>(tags: impl IntoIterator<Item = &'a str>) -> String {
    let links = tags
        .into_iter()
        .map(|tag| {
            format!(
                r#"<a class="badge" href="/tag/{}">{}</a>"#,
                path_segment(tag),
                text(tag)
            )
        })
        .collect::<Vec<_>>();
    if links.is_empty() {
        r#"<span class="muted">None</span>"#.to_string()
    } else {
        links.join(" ")
    }
}

#[derive(Debug, Clone, Default)]
struct GraphTagStats {
    card_count: usize,
    color_counts: [usize; 5],
}

fn tag_graph_stats(catalog: &Catalog) -> HashMap<String, GraphTagStats> {
    let mut stats: HashMap<String, GraphTagStats> = HashMap::new();
    for card in catalog.cards() {
        let mut seen = HashSet::new();
        for tag in card.all_tags() {
            if !seen.insert(tag) {
                continue;
            }
            let entry = stats.entry(tag.to_string()).or_default();
            entry.card_count += 1;
            if let Some(index) = color_index(card.color) {
                entry.color_counts[index] += 1;
            }
        }
    }
    stats
}

fn tag_graph_depths(catalog: &Catalog, edges: &[crate::tags::TagEdge]) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();
    for card in catalog.cards() {
        for tag in &card.tags {
            depths.entry(tag.clone()).or_insert(0);
        }
    }
    for edge in edges {
        depths.entry(edge.child.clone()).or_insert(0);
        depths.entry(edge.parent.clone()).or_insert(0);
    }

    for _ in 0..=edges.len() {
        let mut changed = false;
        for edge in edges {
            let child_depth = depths.get(&edge.child).copied().unwrap_or(0);
            let parent_depth = depths.entry(edge.parent.clone()).or_insert(0);
            if *parent_depth < child_depth + 1 {
                *parent_depth = child_depth + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    depths
}

fn tag_graph_svg(
    catalog: &Catalog,
    inheritance_edges: &[crate::tags::TagEdge],
    tag_stats: &HashMap<String, GraphTagStats>,
    tag_depths: &HashMap<String, usize>,
) -> String {
    let margin = 56_i32;
    let card_color_column_width = 118_i32;
    let card_row_height = 22_i32;
    let tag_column_width = 330_i32;
    let tag_row_height = 42_i32;
    let card_area_width = card_color_column_width * Color::all_displayed().len() as i32;
    let tag_start_x = margin + card_area_width + 180;

    let mut cards_by_color: BTreeMap<u8, Vec<&Card>> = BTreeMap::new();
    for card in catalog.cards() {
        cards_by_color
            .entry(card.color.order())
            .or_default()
            .push(card);
    }
    for cards in cards_by_color.values_mut() {
        cards.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
    }

    let mut tag_layers: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for tag in tag_depths.keys() {
        let depth = tag_depths.get(tag).copied().unwrap_or(0);
        tag_layers.entry(depth).or_default().push(tag.clone());
    }
    for tags in tag_layers.values_mut() {
        tags.sort_by(|left, right| {
            let left_count = tag_stats
                .get(left)
                .map(|stats| stats.card_count)
                .unwrap_or(0);
            let right_count = tag_stats
                .get(right)
                .map(|stats| stats.card_count)
                .unwrap_or(0);
            right_count.cmp(&left_count).then_with(|| left.cmp(right))
        });
    }

    let mut card_positions: HashMap<&str, (i32, i32)> = HashMap::new();
    for (column, color) in Color::all_displayed().iter().enumerate() {
        let Some(cards) = cards_by_color.get(&color.order()) else {
            continue;
        };
        for (index, card) in cards.iter().enumerate() {
            let x = margin + column as i32 * card_color_column_width;
            let y = margin + 60 + index as i32 * card_row_height;
            card_positions.insert(card.name.as_str(), (x, y));
        }
    }

    let mut tag_positions: HashMap<&str, (i32, i32)> = HashMap::new();
    for (depth, tags) in &tag_layers {
        for (index, tag) in tags.iter().enumerate() {
            let x = tag_start_x + *depth as i32 * tag_column_width;
            let y = margin + 60 + index as i32 * tag_row_height;
            tag_positions.insert(tag.as_str(), (x, y));
        }
    }

    let max_card_rows = cards_by_color.values().map(Vec::len).max().unwrap_or(1) as i32;
    let max_tag_rows = tag_layers.values().map(Vec::len).max().unwrap_or(1) as i32;
    let max_depth = tag_layers.keys().copied().max().unwrap_or(0) as i32;
    let width = tag_start_x + (max_depth + 1) * tag_column_width + 260;
    let height =
        margin * 2 + 80 + (max_card_rows * card_row_height).max(max_tag_rows * tag_row_height);

    let card_edges = catalog
        .cards()
        .iter()
        .flat_map(|card| {
            card.tags.iter().filter_map(|tag| {
                let &(card_x, card_y) = card_positions.get(card.name.as_str())?;
                let &(tag_x, tag_y) = tag_positions.get(tag.as_str())?;
                let mid_x = (card_x + tag_x) / 2;
                Some(format!(
                    r#"<path class="graph-card-edge" d="M {} {} C {} {}, {} {}, {} {}" style="--edge-color:{}" />"#,
                    card_x,
                    card_y,
                    mid_x,
                    card_y,
                    mid_x,
                    tag_y,
                    tag_x,
                    tag_y,
                    color_hex(card.color)
                ))
            })
        })
        .collect::<String>();

    let inheritance_lines = inheritance_edges
        .iter()
        .filter_map(|edge| {
            let &(child_x, child_y) = tag_positions.get(edge.child.as_str())?;
            let &(parent_x, parent_y) = tag_positions.get(edge.parent.as_str())?;
            let mid_x = (child_x + parent_x) / 2;
            Some(format!(
                r#"<path class="graph-inheritance-edge" d="M {} {} C {} {}, {} {}, {} {}" />"#,
                child_x, child_y, mid_x, child_y, mid_x, parent_y, parent_x, parent_y
            ))
        })
        .collect::<String>();

    let card_nodes = catalog
        .cards()
        .iter()
        .filter_map(|card| {
            let &(x, y) = card_positions.get(card.name.as_str())?;
            Some(format!(
                r##"<a href="/card/{}"><circle class="graph-card-node" cx="{}" cy="{}" r="5.5" style="--node-fill:{}"><title>{}</title></circle></a>"##,
                path_segment(&card.name),
                x,
                y,
                color_hex(card.color),
                text(&card.name)
            ))
        })
        .collect::<String>();

    let tag_nodes = tag_layers
        .values()
        .flat_map(|tags| tags.iter())
        .filter_map(|tag| {
            let &(x, y) = tag_positions.get(tag.as_str())?;
            let stats = tag_stats.get(tag).cloned().unwrap_or_default();
            let radius = tag_radius(stats.card_count);
            let fill = tag_fill(&stats);
            Some(format!(
                r##"<a href="/tag/{}"><g class="graph-tag-node">
  <circle cx="{}" cy="{}" r="{}" style="--node-fill:{}"><title>{}: {} cards</title></circle>
  <text x="{}" y="{}">{}</text>
</g></a>"##,
                path_segment(tag),
                x,
                y,
                radius,
                fill,
                text(tag),
                stats.card_count,
                x + radius as i32 + 8,
                y + 4,
                text(tag)
            ))
        })
        .collect::<String>();

    let color_labels = Color::all_displayed()
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let x = margin + index as i32 * card_color_column_width - 10;
            format!(
                r#"<text class="graph-column-label" x="{}" y="{}">{}</text>"#,
                x,
                margin + 20,
                color
            )
        })
        .collect::<String>();

    format!(
        r##"<div class="graph-scroll">
  <svg class="tag-graph" viewBox="0 0 {} {}" width="{}" height="{}" role="img" aria-label="Card and tag graph">
    <defs>
      <marker id="graph-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
        <path d="M 0 0 L 10 5 L 0 10 z"></path>
      </marker>
    </defs>
    <text class="graph-section-label" x="{}" y="{}">Cards</text>
    <text class="graph-section-label" x="{}" y="{}">Tags</text>
    <g class="graph-labels">{}</g>
    <g class="graph-card-edges">{}</g>
    <g class="graph-inheritance-edges">{}</g>
    <g class="graph-card-nodes">{}</g>
    <g class="graph-tag-nodes">{}</g>
  </svg>
</div>"##,
        width,
        height,
        width,
        height,
        margin,
        margin,
        tag_start_x,
        margin,
        color_labels,
        card_edges,
        inheritance_lines,
        card_nodes,
        tag_nodes
    )
}

fn top_tag_rows(tag_stats: &HashMap<String, GraphTagStats>) -> String {
    let mut tags = tag_stats.iter().collect::<Vec<_>>();
    tags.sort_by(|left, right| {
        right
            .1
            .card_count
            .cmp(&left.1.card_count)
            .then_with(|| left.0.cmp(right.0))
    });
    tags.into_iter()
        .take(30)
        .map(|(tag, stats)| {
            format!(
                r#"<tr><td><a href="/tag/{}">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                path_segment(tag),
                text(tag),
                stats.card_count,
                color_mix_label(stats)
            )
        })
        .collect()
}

fn color_mix_label(stats: &GraphTagStats) -> String {
    Color::all_displayed()
        .into_iter()
        .zip(stats.color_counts)
        .filter(|(_, count)| *count > 0)
        .map(|(color, count)| format!("{} {}", color, count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn tag_radius(card_count: usize) -> f32 {
    (6.0 + (card_count as f32).sqrt() * 2.7).clamp(7.0, 31.0)
}

fn tag_fill(stats: &GraphTagStats) -> String {
    let total = stats.color_counts.iter().sum::<usize>();
    if total == 0 {
        return "#c9c0b5".to_string();
    }

    let mut red = 0.0_f32;
    let mut green = 0.0_f32;
    let mut blue = 0.0_f32;
    for (color, count) in Color::all_displayed().into_iter().zip(stats.color_counts) {
        let (r, g, b) = color_rgb(color);
        red += r as f32 * count as f32;
        green += g as f32 * count as f32;
        blue += b as f32 * count as f32;
    }
    format!(
        "#{:02x}{:02x}{:02x}",
        (red / total as f32).round() as u8,
        (green / total as f32).round() as u8,
        (blue / total as f32).round() as u8
    )
}

fn color_index(color: Color) -> Option<usize> {
    match color {
        Color::Red => Some(0),
        Color::Green => Some(1),
        Color::Blue => Some(2),
        Color::Purple => Some(3),
        Color::Colorless => Some(4),
        Color::Other => None,
    }
}

fn color_hex(color: Color) -> &'static str {
    match color {
        Color::Red => "#c94a3c",
        Color::Green => "#3f9b67",
        Color::Blue => "#3f7db7",
        Color::Purple => "#7a5ab7",
        Color::Colorless => "#b8aea0",
        Color::Other => "#777777",
    }
}

fn color_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Red => (201, 74, 60),
        Color::Green => (63, 155, 103),
        Color::Blue => (63, 125, 183),
        Color::Purple => (122, 90, 183),
        Color::Colorless => (184, 174, 160),
        Color::Other => (119, 119, 119),
    }
}

fn query_chip(query: &str) -> String {
    format!(
        r#"<a href="/search?{}">{}</a>"#,
        query_string(&[("q", query)]),
        text(query)
    )
}

fn view_link(
    params: &HashMap<String, String>,
    current: ViewMode,
    target: ViewMode,
    label: &str,
) -> String {
    format!(
        r#"<a class="{}" href="{}">{}</a>"#,
        if current == target { "active" } else { "" },
        attr(&updated_query(
            params,
            &[("view", target.key()), ("page", "1")]
        )),
        label
    )
}

fn hidden_inputs(params: &HashMap<String, String>, exclude: &[&str]) -> String {
    params
        .iter()
        .filter(|(key, value)| !exclude.contains(&key.as_str()) && !value.is_empty())
        .map(|(key, value)| {
            format!(
                r#"<input type="hidden" name="{}" value="{}">"#,
                attr(key),
                attr(value)
            )
        })
        .collect()
}

fn updated_query(params: &HashMap<String, String>, updates: &[(&str, &str)]) -> String {
    let mut next = params.clone();
    for (key, value) in updates {
        next.insert((*key).to_string(), (*value).to_string());
    }
    format!("/search?{}", query_map(&next))
}

fn query_map(params: &HashMap<String, String>) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    let mut entries: Vec<_> = params
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    for (key, value) in entries {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn query_string(pairs: &[(&str, &str)]) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        if !value.is_empty() {
            serializer.append_pair(key, value);
        }
    }
    serializer.finish()
}

fn selected(value: bool) -> &'static str {
    if value { "selected" } else { "" }
}

fn badge(value: &str) -> String {
    format!(r#"<span class="badge">{}</span>"#, text(value))
}

fn cost_label(card: &Card) -> String {
    let cost = card.cost.to_string();
    if cost.is_empty() {
        "no cost".to_string()
    } else {
        cost
    }
}

fn path_segment(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn text(value: &str) -> String {
    encode_safe(value).to_string()
}

fn attr(value: &str) -> String {
    encode_double_quoted_attribute(value).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Catalog;

    #[test]
    fn renders_known_card_page() {
        let catalog = Catalog::load("assets/spire_tagger.db").expect("catalog");
        let html = card_page(&catalog, "Bash").expect("bash page");
        assert!(html.contains("Bash"));
        assert!(html.contains("apply 2"));
    }

    #[test]
    fn renders_tag_graph_page() {
        let catalog = Catalog::load("assets/spire_tagger.db").expect("catalog");
        let html = tag_graph_page(&catalog);
        assert!(html.contains("Tag Graph"));
        assert!(html.contains(r#"class="tag-graph""#));
        assert!(html.contains("graph-card-node"));
        assert!(html.contains("graph-tag-node"));
        assert!(html.contains("graph-card-edge"));
        assert!(html.contains("graph-inheritance-edge"));
        assert!(html.contains("copy-self"));
    }
}
