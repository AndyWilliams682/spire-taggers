//! Spire Tagger core library.
//!
//! This crate contains the logic for loading the Slay the Spire catalog from a database,
//! modelling the domain (cards and tags), parsing search queries, and rendering the web UI.

pub mod data;
pub mod domain;
pub mod query;
pub mod render;
pub mod tags;
pub mod web;

use data::Catalog;
use std::sync::Arc;

/// The global application state shared across all HTTP handlers.
/// 
/// We wrap the `Catalog` in an `Arc` (Atomically Reference Counted) smart pointer 
/// so that it can be safely and cheaply cloned for each incoming request.
#[derive(Clone)]
pub struct AppState {
    pub catalog: Arc<Catalog>,
}

impl AppState {
    /// Creates a new `AppState` with the given catalog.
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self { catalog }
    }
}
