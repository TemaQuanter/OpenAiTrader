// This file contains the functionality to run a server.

use std::sync::Arc;

use axum::{http::Method, routing::put, Router};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use teloxide::{adaptors::Throttle, Bot};
use tower_http::cors::{Any, CorsLayer};

use crate::utils::types_and_constants::SERVER_ADDRESS;

use super::routes::{approve::approve, expired::expired};

/// This structure is an application state.
/// (It is necessary to share database connection)
#[derive(Clone)]
pub struct AppState {
    // This is a pool of connections to the database.
    pub pool: Pool<ConnectionManager<PgConnection>>,
    // Bot to send notifications to users.
    pub bot: Throttle<Bot>,
} // end struct AppState

/// This function creates an application state.
fn create_app_state(pool: Pool<ConnectionManager<PgConnection>>, bot: Throttle<Bot>) -> AppState {
    AppState { pool, bot } // end AppState
} // end fn create_app_state

/// This function assembles the main router for the server.
async fn create_app(pool: Pool<ConnectionManager<PgConnection>>, bot: Throttle<Bot>) -> Router {
    // Get the main app state.
    let app_state = Arc::new(create_app_state(pool, bot));

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
pub async fn run_server(pool: Pool<ConnectionManager<PgConnection>>, bot: Throttle<Bot>) {
    // Assemble a router for the server.
    let app = create_app(pool, bot).await;

    // Run a server at the specified address.
    let listener = tokio::net::TcpListener::bind(SERVER_ADDRESS).await.unwrap();
    axum::serve(listener, app).await.unwrap();
} // end fn run_server
