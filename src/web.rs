//! Web routing and handlers.
//!
//! This module defines the Axum HTTP router and mapping of URLs to the specific handlers 
//! that execute queries and delegate to the `render` module to generate HTML.

use crate::{AppState, render};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use std::collections::HashMap;

/// Builds the Axum router for the application, registering all web endpoints.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/search", get(search))
        .route("/advanced", get(advanced))
        .route("/syntax", get(syntax))
        .route("/tags", get(tags))
        .route("/tag-graph", get(tag_graph))
        .route("/tag/:tag", get(tag))
        .route("/card/:name", get(card))
        .route("/random", get(random))
        .route("/static/site.css", get(css))
        .with_state(state)
}

async fn home(State(state): State<AppState>) -> Html<String> {
    Html(render::home(&state.catalog))
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    Html(render::results(&state.catalog, &params))
}

async fn advanced(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    Html(render::advanced_page(&params))
}

async fn syntax() -> Html<String> {
    Html(render::syntax_page())
}

async fn tags(State(state): State<AppState>) -> Html<String> {
    Html(render::tags_page(&state.catalog))
}

async fn tag_graph(State(state): State<AppState>) -> Html<String> {
    Html(render::tag_graph_page(&state.catalog))
}

async fn tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    Html(render::tag_page(&state.catalog, &tag, &params))
}

async fn card(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    match render::card_page(&state.catalog, &name) {
        Some(html) => Html(html).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Html(render::error_page(404, "Card not found")),
        )
            .into_response(),
    }
}

async fn random(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    match render::random_card(&state.catalog, &params) {
        Some(render::Rendered::Redirect(location)) => {
            Redirect::temporary(&location).into_response()
        }
        Some(render::Rendered::Html(html)) => Html(html).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Html(render::error_page(
                404,
                "No random card matched that search.",
            )),
        )
            .into_response(),
    }
}

async fn css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        render::CSS,
    )
        .into_response()
}
