// This file contains the functionality to run a server.

use std::sync::Arc;

use axum::{
    http::Method,
    routing::{get, put},
    Router,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use tower_http::cors::{Any, CorsLayer};

use crate::utils::types_and_constants::SERVER_ADDRESS;

use super::routes::{approve::approve, expired::expired};

/// This structure is an application state.
/// (It is necessary to share database connection)
#[derive(Clone)]
pub struct AppState {
    // This is a pool of connections to the database.
    pub pool: Pool<ConnectionManager<PgConnection>>,
} // end struct AppState

/// This function creates an application state.
fn create_app_state(pool: Pool<ConnectionManager<PgConnection>>) -> AppState {
    AppState { pool } // end AppState
} // end fn create_app_state

/// This function assembles the main router for the server.
async fn create_app(pool: Pool<ConnectionManager<PgConnection>>) -> Router {
    // Get the main app state.
    let app_state = Arc::new(create_app_state(pool));

    // Create the main server router.
    let router = Router::new()
        // This route approves users in the bot.
        .route("/approve/:telegram_id", put(approve))
        // This route sets the subscription status of the user
        // to expired.
        .route("/expired/:telegram_id", put(expired))
        // Required to make it possible working through HTTPS.
        .layer(
            CorsLayer::new()
                // Allow requests from any origin.
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        )
        .with_state(app_state);

    router
} // end fn create_app

/// This function runs a server.
pub async fn run_server(pool: Pool<ConnectionManager<PgConnection>>) {
    // Assemble a router for the server.
    let app = create_app(pool).await;

    // Run a server at the specified address.
    let listener = tokio::net::TcpListener::bind(SERVER_ADDRESS).await.unwrap();
    axum::serve(listener, app).await.unwrap();
} // end fn run_server
