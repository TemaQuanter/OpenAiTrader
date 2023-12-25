use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{ChatId, ParseMode},
};

use crate::{
    db_objects::{status_model::StatusId, user_model::User},
    handlers::server::run::AppState,
    utils::{database_service::DatabaseService, lazy_static::SUBSCRIPTION_BOT_URL},
};

/// This endpoint is used to indicate that a user's subscription expired.
pub async fn expired(
    Path(telegram_id): Path<i64>,
    State(app_state): State<Arc<AppState>>,
) -> Result<(), StatusCode> {
    // Get a connection to the database.
    let mut conn = match DatabaseService::get_pool_connection(&app_state.pool) {
        Ok(conn) => conn,
        Err(error) => {
            log::error!("{}", error.to_string());
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        } // end Err
    }; // end match

    // Shadow for convenience.
    let conn = &mut conn;

    // Try get the user from the database.
    let mut user = match User::get_from_db_by_id(telegram_id, conn) {
        Ok(user) => user,
        Err(error) => {
            log::error!("{}", error);

            // If an error occurred at this stage, the function cannot
            // proceed.
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        } // end Err
    }; // end match

    // Check whether or not their status is rejected or banned.
    if user.status_id == StatusId::Rejected as i32 || user.status_id == StatusId::Banned as i32 {
        // Not allowed to be changed.
        return Err(StatusCode::FORBIDDEN);
    } // end if

    // Mark the user subscription as expired the user.
    user.status_id = StatusId::Expired as i32;

    // Update the user in the database.
    let res = User::update(&user, conn);

    // Check that the update was successful.
    if res.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    } // end if

    // Inform a user that their subscription expired.
    app_state
        .bot
        .send_message(
            ChatId(telegram_id),
            format!("Your subscription <b>expired</b>.\n\nPlease, renew it before you can use me again.\n\n{}", *SUBSCRIPTION_BOT_URL),
        )
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|error| {
            log::error!("{}", error.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    log::info!("Expired: {}", telegram_id);
    Ok(())
} // end fn expired
