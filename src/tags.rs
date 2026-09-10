//! Tag inheritance logic.
//!
//! This module defines the `TagGraph`, a directed graph used to model how some tags
//! inherently imply other tags (e.g., if a card has `apply-vulnerable`, it should
//! also be found when searching for `apply-debuff`).

use std::collections::{HashMap, HashSet, VecDeque};

/// Represents a single directed edge in the tag inheritance graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEdge {
    pub child: String,
    pub parent: String,
}

const BUFFS: &[&str] = &[
    "strength",
    "dexterity",
    "focus",
    "orb_slot",
    "intangible",
    "artifact",
    "plated_armor",
    "mantra",
];

const DEBUFFS: &[&str] = &["vulnerable", "weak", "frail", "mark", "lock_on", "poison"];

const STATUSES: &[&str] = &["burn", "wound", "void", "dazed"];

const TOKENS: &[&str] = &[
    "shiv",
    "smite",
    "safety",
    "miracle",
    "through_violence",
    "insight",
    "beta",
    "omega",
    "expunger",
    "shiv\\+",
    "smite\\+",
    "safety\\+",
    "miracle\\+",
    "through_violence\\+",
    "insight\\+",
    "beta\\+",
    "omega\\+",
    "expunger\\+",
];

const ADD_TARGETS: &[&str] = &[
    "copy",
    "random_common",
    "self",
    "random_colorless",
    "random_attack",
    "random_skill",
    "random_power",
    "random_status",
    "random_curse",
];

const TRIGGER_TAGS: &[&str] = &[
    "draw-self",
    "hp-cost",
    "gain-block",
    "you-are-attacked",
    "take-unblocked-damage",
    "attack-targeted",
    "apply-debuff",
    "stance-change",
    "scry",
    "wrath",
    "unblocked-attack",
    "fatal",
    "retain-self",
    "play-attack",
    "play-skill",
    "play-power",
    "play-status",
    "play-curse",
    "play",
    "draw-attack",
    "draw-skill",
    "draw-power",
    "draw-status",
    "draw-curse",
    "draw",
    "exhaust-other",
    "retain-other",
    "create-other",
    "draw-self-this-turn",
    "hp-cost-this-turn",
    "gain-block-this-turn",
    "you-are-attacked-this-turn",
    "take-unblocked-damage-this-turn",
    "attack-targeted-this-turn",
    "apply-debuff-this-turn",
    "stance-change-this-turn",
    "scry-this-turn",
    "wrath-this-turn",
    "unblocked-attack-this-turn",
    "fatal-this-turn",
    "retain-self-this-turn",
];

/// A directed acyclic graph representing tag inheritance.
/// Nodes are tags, and edges point from a child tag to a parent tag.
#[derive(Debug, Clone, Default)]
pub struct TagGraph {
    /// Maps a child tag to a list of its parent tags.
    parents: HashMap<String, Vec<String>>,
}

impl TagGraph {
    pub fn spire_defaults() -> Self {
        let mut graph = Self::default();
        for (child, parent) in [
            ("apply-vulnerable", "opener"),
            ("opener", "sequence-matters"),
            ("copy-self", "copy"),
            ("copy", "add-card"),
            ("add-card", "fatten"),
            ("hp-cost-trigger", "hp-cost-matters"),
            ("play-from-top", "play-free"),
            ("exhaust-hand", "exhaust-other"),
            ("card-draw-triggered", "card-draw"),
        ] {
            graph.add(child, parent);
        }

        for debuff in DEBUFFS {
            graph.add(format!("apply-{debuff}"), "apply-debuff");
        }
        for buff in BUFFS {
            graph.add(format!("gain-{buff}"), "gain-buff");
            graph.add(format!("lose-{buff}"), "lose-buff");
            graph.add(format!("nerf-{buff}"), "nerf");
        }
        for status in STATUSES {
            graph.add(format!("add-{status}"), "fatten");
        }
        for token in TOKENS {
            graph.add(format!("add-{token}"), "fatten");
        }
        for target in ADD_TARGETS {
            graph.add(format!("add-{target}"), "fatten");
        }
        for trigger in TRIGGER_TAGS {
            graph.add(format!("{trigger}-trigger"), "trigger-clause");
        }
        graph
    }

    /// Adds a directed edge from `child` to `parent` in the tag graph.
    pub fn add(&mut self, child: impl Into<String>, parent: impl Into<String>) {
        self.parents
            .entry(child.into())
            .or_default()
            .push(parent.into());
    }

    pub fn parents(&self, tag: &str) -> &[String] {
        self.parents.get(tag).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn children_of(&self, parent: &str) -> Vec<&str> {
        let mut children: Vec<_> = self
            .parents
            .iter()
            .filter_map(|(child, parents)| {
                parents
                    .iter()
                    .any(|candidate| candidate == parent)
                    .then_some(child.as_str())
            })
            .collect();
        children.sort_unstable();
        children
    }

    pub fn edges(&self) -> Vec<TagEdge> {
        let mut edges = self
            .parents
            .iter()
            .flat_map(|(child, parents)| {
                parents.iter().map(|parent| TagEdge {
                    child: child.clone(),
                    parent: parent.clone(),
                })
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            left.parent
                .cmp(&right.parent)
                .then_with(|| left.child.cmp(&right.child))
        });
        edges
    }

    pub fn node_count(&self) -> usize {
        let mut nodes = HashSet::new();
        for edge in self.edges() {
            nodes.insert(edge.child);
            nodes.insert(edge.parent);
        }
        nodes.len()
    }

    /// Computes all inherited parent tags for a given set of explicit tags using Breadth-First Search.
    /// This includes transitively inherited tags.
    pub fn inherited_for<'a>(&self, tags: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut inherited = Vec::new();
        let mut frontier = VecDeque::new();

        for tag in tags {
            seen.insert(tag.to_string());
            frontier.push_back(tag.to_string());
        }

        while let Some(tag) = frontier.pop_front() {
            for parent in self.parents(&tag) {
                if seen.insert(parent.clone()) {
                    inherited.push(parent.clone());
                    frontier.push_back(parent.clone());
                }
            }
        }

        inherited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherits_transitively() {
        let graph = TagGraph::spire_defaults();
        let inherited = graph.inherited_for(["copy-self"]);
        assert!(inherited.contains(&"copy".to_string()));
        assert!(inherited.contains(&"add-card".to_string()));
        assert!(inherited.contains(&"fatten".to_string()));
    }

    #[test]
    fn exposes_graph_edges() {
        let graph = TagGraph::spire_defaults();
        assert!(
            graph
                .edges()
                .iter()
                .any(|edge| edge.child == "copy-self" && edge.parent == "copy")
        );
        assert!(graph.node_count() > 50);
    }
}
