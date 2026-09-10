//! Data loading and caching layer.
//!
//! This module is responsible for loading the catalog of cards and tags from the embedded
//! SQLite database into memory, and storing them in a `Catalog` for fast, in-memory queries.

use crate::{
    domain::{Card, CardKind, Color, EnergyCost, Rarity},
    tags::TagGraph,
};
use anyhow::Context;
use rusqlite::{Connection, types::ValueRef};
use std::{collections::HashMap, path::Path};

/// In-memory catalog containing all cards, tags, and the tag inheritance graph.
#[derive(Debug, Clone)]
pub struct Catalog {
    cards: Vec<Card>,
    tags: Vec<String>,
    tag_graph: TagGraph,
}

impl Catalog {
    /// Connects to the SQLite database at `path` and loads the entire catalog into memory.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        let tag_graph = TagGraph::spire_defaults();
        let tags_by_card = load_card_tags(&conn).context("failed to load card tags")?;
        let mut cards =
            load_cards(&conn, &tags_by_card, &tag_graph).context("failed to load cards")?;
        let tags = load_tags(&conn).context("failed to load tags")?;

        cards.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });

        Ok(Self {
            cards,
            tags,
            tag_graph,
        })
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn tag_graph(&self) -> &TagGraph {
        &self.tag_graph
    }

    /// Finds a card by its exact name, ignoring case.
    pub fn find_card(&self, name: &str) -> Option<&Card> {
        self.cards
            .iter()
            .find(|card| card.name.eq_ignore_ascii_case(name))
    }

    pub fn cards_in_display_order(&self) -> Vec<&Card> {
        let mut cards: Vec<_> = self.cards.iter().collect();
        cards.sort_by(|left, right| {
            (
                left.color.order(),
                left.rarity.order(),
                left.name.to_ascii_lowercase(),
            )
                .cmp(&(
                    right.color.order(),
                    right.rarity.order(),
                    right.name.to_ascii_lowercase(),
                ))
        });
        cards
    }
}

/// Helper to load cards from the `card` table and assign them their direct and inherited tags.
fn load_cards(
    conn: &Connection,
    tags_by_card: &HashMap<String, Vec<String>>,
    tag_graph: &TagGraph,
) -> anyhow::Result<Vec<Card>> {
    let mut stmt = conn.prepare(
        "select name, color, type, rarity, cost, body, multiplayer from card order by name",
    )?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let cost = match row.get_ref(4)? {
            ValueRef::Null => None,
            ValueRef::Integer(value) => Some(value.to_string()),
            ValueRef::Real(value) => Some(value.to_string()),
            ValueRef::Text(value) => Some(String::from_utf8_lossy(value).into_owned()),
            ValueRef::Blob(_) => None,
        };
        let direct_tags = tags_by_card.get(&name).cloned().unwrap_or_default();
        let inherited_tags = tag_graph.inherited_for(direct_tags.iter().map(String::as_str));
        Ok(Card {
            name,
            color: Color::parse(&row.get::<_, String>(1)?),
            kind: CardKind::parse(&row.get::<_, String>(2)?),
            rarity: Rarity::parse(&row.get::<_, String>(3)?),
            cost: EnergyCost::parse(cost),
            body: row.get(5)?,
            multiplayer: row.get::<_, i64>(6)? != 0,
            tags: direct_tags,
            inherited_tags,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_card_tags(conn: &Connection) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare("select card, tag from card_tags order by tag")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut tags_by_card: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (card, tag) = row?;
        tags_by_card.entry(card).or_default().push(tag);
    }
    Ok(tags_by_card)
}

fn load_tags(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare("select name from tags order by name")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_bundled_database() {
        let catalog = Catalog::load("assets/spire_tagger.db").expect("catalog loads");
        assert!(catalog.cards().len() > 300);
        assert!(catalog.tags().len() > 300);
        assert!(catalog.find_card("Bash").is_some());
    }
}
