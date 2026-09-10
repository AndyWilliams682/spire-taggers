//! Domain models for the Slay the Spire card catalog.
//!
//! This module defines the core data structures used to represent cards, their properties
//! (Color, Type/Kind, Rarity, Energy Cost), and sorting rules.

use std::{cmp::Ordering, fmt};

/// Represents the character class color or overarching category of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Green,
    Blue,
    Purple,
    Colorless,
    Unimplemented,
}

impl Color {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "red" => Self::Red,
            "green" => Self::Green,
            "blue" => Self::Blue,
            "purple" => Self::Purple,
            "colorless" => Self::Colorless,
            _ => Self::Unimplemented,
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Self::Red => 0,
            Self::Green => 1,
            Self::Blue => 2,
            Self::Purple => 3,
            Self::Colorless => 4,
            Self::Unimplemented => 99,
        }
    }

    // TODO: What does this function do?
    pub fn all_displayed() -> [Self; 5] {
        [
            Self::Red,
            Self::Green,
            Self::Blue,
            Self::Purple,
            Self::Colorless,
        ]
    }

    // TODO: Add function to get the class name from the color
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Colorless => "colorless",
            Self::Unimplemented => "unimplemented",
        })
    }
}

/// Represents the type of card (e.g. Attack, Skill, Power, Status, Curse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardKind { // TODO: Rename to CardType
    Attack,
    Skill,
    Power,
    Status,
    Curse,
    // TODO: Add STS2 types
    Unimplemented, // TODO: Is this really needed?
}


/*
TODO: Could I metaprogram something to capture these types?
Color, CardType, Rarity all implement the same methods:
    Parse from string, display as string
    Order
They could all literally be defined just using a vector of strings
    The rest of the code could generate itself
What would I call this? Do I need to give it a name?
Does this make programming the rest of the code a pain in the ass?
CardProperty Trait? CardStringProperty trait?
    Other props are name, body, upgraded body, cost
*/

impl CardKind { // TODO: Could I metaprogram something to capture these types?
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "attack" => Self::Attack,
            "skill" => Self::Skill,
            "power" => Self::Power,
            "status" => Self::Status,
            "curse" => Self::Curse,
            _ => Self::Unimplemented,
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Self::Attack => 0,
            Self::Skill => 1,
            Self::Power => 2,
            Self::Status => 3,
            Self::Curse => 4,
            Self::Unimplemented => 99,
        }
    }
}

impl fmt::Display for CardKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Attack => "attack",
            Self::Skill => "skill",
            Self::Power => "power",
            Self::Status => "status",
            Self::Curse => "curse",
            Self::Unimplemented => "unimplemented",
        })
    }
}

/// Represents the rarity of a card (e.g., Common, Uncommon, Rare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Unimplemented, // TODO: Is this really needed?
}

impl Rarity {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "basic" => Self::Basic,
            "common" => Self::Common,
            "uncommon" => Self::Uncommon,
            "rare" => Self::Rare,
            "special" => Self::Special,
            _ => Self::Unimplemented,
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Self::Basic => 0,
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::Special => 4,
            Self::Unimplemented => 99,
        }
    }
}

impl fmt::Display for Rarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Basic => "basic",
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Special => "special",
            Self::Unimplemented => "unimplemented",
        })
    }
}

/*
Need to split EnergyCost out into two classes:
1) A value class that can be any integer, or X
2) A cost class that holds values for different things (Energy and Stars)
    Parse the cost string or whatever
*/

/// Represents the energy cost of a card.
/// Slay the Spire cards can have numeric costs, dynamic 'X' costs, be missing (unplayable), or have special costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyCost { // TODO: Rename to Cost
    Number(u16, String), // TODO: Remove the string here, it is useless; TODO: Rename to EnergyValue
    X(String),
    Missing, // TODO: Remove, replace with Option<EnergyCost>? Or rename to Unplayable
    Unimplemented(String), // TODO: Is this really needed?
    // TODO: Add type here for star cost (with two numbers)
    // 
}

impl EnergyCost {
    pub fn parse(value: Option<String>) -> Self {
        let Some(value) = value else {
            return Self::Missing;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Self::Missing;
        }
        if let Some(number) = leading_number(trimmed) {
            return Self::Number(number, trimmed.to_string());
        }
        if trimmed.to_ascii_lowercase().contains('x') {
            return Self::X(trimmed.to_string());
        }
        Self::Unimplemented(trimmed.to_string())
    }

    pub fn sort_key(&self) -> (u16, &str) {
        match self {
            Self::Number(value, raw) => (*value, raw.as_str()),
            Self::X(raw) => (50, raw.as_str()), // TODO: Order as 0? That's how magic does it
            Self::Unimplemented(raw) => (100, raw.as_str()),
            Self::Missing => (999, ""),
        }
    }
}

impl fmt::Display for EnergyCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(_, raw) | Self::X(raw) | Self::Unimplemented(raw) => f.write_str(raw),
            Self::Missing => Ok(()),
        }
    }
}

fn leading_number(value: &str) -> Option<u16> {
    let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// The core domain entity representing a single Slay the Spire card.
#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub color: Color,
    pub kind: CardKind,
    pub rarity: Rarity,
    pub cost: EnergyCost,
    pub body: String, // TODO: Replace as base_body
    // TODO: add pub upgraded_body: String for the upgraded version of the card
        // Need to handle edge case of repeatable upgrade cards
    pub multiplayer: bool,
    /// Tags explicitly applied to this card.
    pub tags: Vec<String>,
    /// Tags inherited by this card due to the TagGraph (e.g. `apply-vulnerable` -> `apply-debuff`).
    pub inherited_tags: Vec<String>,
    /*
    TODO: how is upgrade info stored in the database?
        Many cards just have a value change
        Some cards have text added or removed
            Added text could be easily captured
            Removed text... not as much
            Altered text, completely fucked
        One card can be upgraded multiple times, complicating things
            This case could still be displayed as a single upgraded string?
    */
}

impl Card {
    pub fn all_tags(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .chain(self.inherited_tags.iter())
            .map(String::as_str)
    }
}

/// Trait for defining how to compare and sort cards.
/// TODO: How is this used?
pub trait CardOrder {
    fn compare(&self, left: &Card, right: &Card) -> Ordering;
}

/// The available sorting modes that a user can select to sort the card catalog or search results.
/// TODO: Understand, re-implement, layered sorts?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Color,
    Rarity,
    Name,
    Type,
    Cost,
}

impl SortMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "rarity" => Self::Rarity,
            "name" => Self::Name,
            "type" => Self::Type,
            "cost" => Self::Cost,
            _ => Self::Color,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Rarity => "rarity",
            Self::Name => "name",
            Self::Type => "type",
            Self::Cost => "cost",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Color => "Color",
            Self::Rarity => "Rarity",
            Self::Name => "Name",
            Self::Type => "Type",
            Self::Cost => "Energy cost",
        }
    }

    pub fn all() -> [Self; 5] {
        [
            Self::Color,
            Self::Rarity,
            Self::Name,
            Self::Type,
            Self::Cost,
        ]
    }
}

impl CardOrder for SortMode {
    fn compare(&self, left: &Card, right: &Card) -> Ordering {
        match self {
            Self::Color => (
                left.color.order(),
                left.rarity.order(),
                left.name.to_ascii_lowercase(),
            )
                .cmp(&(
                    right.color.order(),
                    right.rarity.order(),
                    right.name.to_ascii_lowercase(),
                )),
            Self::Rarity => (
                left.rarity.order(),
                left.color.order(),
                left.name.to_ascii_lowercase(),
            )
                .cmp(&(
                    right.rarity.order(),
                    right.color.order(),
                    right.name.to_ascii_lowercase(),
                )),
            Self::Name => left
                .name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase()),
            Self::Type => (left.kind.order(), left.name.to_ascii_lowercase())
                .cmp(&(right.kind.order(), right.name.to_ascii_lowercase())),
            Self::Cost => (left.cost.sort_key(), left.name.to_ascii_lowercase())
                .cmp(&(right.cost.sort_key(), right.name.to_ascii_lowercase())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compound_numeric_costs_by_leading_number() {
        assert_eq!(EnergyCost::parse(Some("1 (0)".to_string())).sort_key().0, 1);
    }
}
