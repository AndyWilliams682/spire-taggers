//! Query and search logic.
//!
//! This module contains the parsing and execution logic for searching the card catalog.
//! It supports advanced filtering by name, cost, type, rarity, and specific tags.

use crate::domain::{Card, CardOrder, EnergyCost, SortMode};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

impl Direction {
    pub fn parse(value: &str) -> Self {
        match value {
            "desc" => Self::Desc,
            _ => Self::Asc,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Images,
    Text,
    Checklist,
}

impl ViewMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "text" => Self::Text,
            "checklist" => Self::Checklist,
            _ => Self::Images,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Images => "images",
            Self::Text => "text",
            Self::Checklist => "checklist",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Name,
    Body,
    Tag,
    Type,
    Color,
    Rarity,
    Cost,
    Unknown(String),
}

impl Field {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "name" | "n" => Self::Name,
            "body" | "o" | "oracle" => Self::Body,
            "tag" => Self::Tag,
            "type" | "t" => Self::Type,
            "color" | "c" => Self::Color,
            "rarity" | "r" => Self::Rarity,
            "cost" | "mana" => Self::Cost,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    negated: bool,
    field: Option<Field>,
    value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardQuery {
    terms: Vec<Term>,
}

impl CardQuery {
    pub fn parse(input: &str) -> Self {
        let raw_terms = shell_words::split(input).unwrap_or_else(|_| {
            input
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        });
        let terms = raw_terms
            .into_iter()
            .filter_map(|mut raw| {
                let negated = raw.starts_with('-');
                if negated {
                    raw.remove(0);
                }
                if raw.is_empty() {
                    return None;
                }
                let (field, value) = raw
                    .split_once(':')
                    .map(|(field, value)| (Some(Field::parse(field)), value.to_string()))
                    .unwrap_or((None, raw));
                Some(Term {
                    negated,
                    field,
                    value,
                })
            })
            .collect();
        Self { terms }
    }

    pub fn matches(&self, card: &Card) -> bool {
        self.terms.iter().all(|term| {
            let matched = match &term.field {
                Some(field) => field_matches(card, field, &term.value),
                None => plain_text_matches(card, &term.value),
            };
            if term.negated { !matched } else { matched }
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query_text: String,
    pub query: CardQuery,
    pub sort: SortMode,
    pub direction: Direction,
    pub view: ViewMode,
    pub page: usize,
    pub per_page: usize,
    pub columns: Vec<String>,
    pub include_multiplayer: bool,
}

impl SearchOptions {
    pub fn from_params(params: &HashMap<String, String>) -> Self {
        let query_text = build_advanced_query(params);
        let page = parse_clamped_usize(params.get("page"), 1, 100_000, 1);
        let per_page = parse_clamped_usize(params.get("per_page"), 12, 120, 36);
        let columns = params
            .get("columns")
            .map(|value| split_columns(value))
            .unwrap_or_else(|| split_columns("damage,block,card-draw"));

        Self {
            query: CardQuery::parse(&query_text),
            query_text,
            sort: SortMode::parse(params.get("sort").map(String::as_str).unwrap_or("color")),
            direction: Direction::parse(params.get("dir").map(String::as_str).unwrap_or("asc")),
            view: ViewMode::parse(params.get("view").map(String::as_str).unwrap_or("images")),
            page,
            per_page,
            columns,
            include_multiplayer: params.get("multiplayer").is_some_and(|value| value == "on"),
        }
    }

    pub fn filter<'a>(&self, cards: impl IntoIterator<Item = &'a Card>) -> Vec<&'a Card> {
        let mut filtered: Vec<_> = cards
            .into_iter()
            .filter(|card| self.include_multiplayer || !card.multiplayer)
            .filter(|card| self.query.matches(card))
            .collect();
        filtered.sort_by(|left, right| self.sort.compare(left, right));
        if self.direction == Direction::Desc {
            filtered.reverse();
        }
        filtered
    }
}

pub fn build_advanced_query(params: &HashMap<String, String>) -> String {
    let mut parts = Vec::new();
    if let Some(base) = params.get("q").filter(|value| !value.trim().is_empty()) {
        parts.push(base.trim().to_string());
    }
    for field in ["name", "body", "tag", "color", "type", "rarity", "cost"] {
        if let Some(value) = params.get(field).filter(|value| !value.trim().is_empty()) {
            let trimmed = value.trim();
            if trimmed.contains(' ') {
                parts.push(format!("{field}:\"{trimmed}\""));
            } else {
                parts.push(format!("{field}:{trimmed}"));
            }
        }
    }
    parts.join(" ")
}

fn split_columns(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(12)
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_clamped_usize(value: Option<&String>, min: usize, max: usize, default: usize) -> usize {
    value
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn plain_text_matches(card: &Card, value: &str) -> bool {
    let needle = value.to_ascii_lowercase();
    card.name.to_ascii_lowercase().contains(&needle)
}

fn field_matches(card: &Card, field: &Field, value: &str) -> bool {
    let needle = value.trim().to_ascii_lowercase();
    match field {
        Field::Name => card.name.to_ascii_lowercase().contains(&needle),
        Field::Body => card.body.to_ascii_lowercase().contains(&needle),
        Field::Tag => card
            .all_tags()
            .any(|tag| tag.to_ascii_lowercase().contains(&needle)),
        Field::Type => card.kind.to_string().contains(&needle),
        Field::Color => card.color.to_string().contains(&needle),
        Field::Rarity => card.rarity.to_string().contains(&needle),
        Field::Cost => cost_matches(&card.cost, &needle),
        Field::Unknown(_) => false,
    }
}

fn cost_matches(cost: &EnergyCost, value: &str) -> bool {
    if let Some(target) = value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse::<u16>()
        .ok()
    {
        let actual = cost.sort_key().0;
        if value.starts_with("<=") {
            return actual <= target;
        }
        if value.starts_with(">=") {
            return actual >= target;
        }
        if value.starts_with('<') {
            return actual < target;
        }
        if value.starts_with('>') {
            return actual > target;
        }
    }
    cost.to_string().to_ascii_lowercase().contains(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CardKind, Color, Rarity};

    fn bash() -> Card {
        Card {
            name: "Bash".to_string(),
            color: Color::Red,
            kind: CardKind::Attack,
            rarity: Rarity::Basic,
            cost: EnergyCost::Number(2, "2".to_string()),
            body: "deal 8 damage. apply 2 vulnerable.".to_string(),
            multiplayer: false,
            tags: vec!["damage".to_string(), "apply-vulnerable".to_string()],
            inherited_tags: vec!["apply-debuff".to_string()],
        }
    }

    #[test]
    fn supports_negated_field_terms() {
        assert!(CardQuery::parse("tag:damage -tag:fatten").matches(&bash()));
        assert!(!CardQuery::parse("tag:damage -tag:apply").matches(&bash()));
    }

    #[test]
    fn supports_cost_comparisons() {
        assert!(CardQuery::parse("cost:>=2").matches(&bash()));
        assert!(!CardQuery::parse("cost:<2").matches(&bash()));
    }

    #[test]
    fn bare_terms_only_match_card_names() {
        assert!(CardQuery::parse("bash").matches(&bash()));
        assert!(!CardQuery::parse("vulnerable").matches(&bash()));
        assert!(!CardQuery::parse("damage").matches(&bash()));
        assert!(CardQuery::parse("body:vulnerable").matches(&bash()));
        assert!(CardQuery::parse("tag:damage").matches(&bash()));
    }
}
