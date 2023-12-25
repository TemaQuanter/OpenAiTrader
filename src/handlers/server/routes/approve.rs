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
    utils::database_service::DatabaseService,
};

/// This endpoint is used to approve users or add new ones to the bot.
pub async fn approve(
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

    // Try insert a new approved user.
    let res = User::insert_into_db(
        &User {
            telegram_id,
            telegram_username: None,
            status_id: 3,
            exchange_api_key: None,
            exchange_api_secret: None,
            exchange_api_data_validated: false,
            deposit: None,
            risk: None,
            bot_active: false,
            role_id: 1,
        },
        conn,
    ); // end User

    // If the operation was successful, the function is done.
    if res.is_ok() {
        return Ok(());
    } // end if

    // Otherwise the user already exists and it is necessary to modify them.

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
        // Not allow the user to get approved.
        return Err(StatusCode::FORBIDDEN);
    } // end if

    // Approve the user.
    user.status_id = StatusId::Approved as i32;

    // Update the user in the database.
    let res = User::update(&user, conn);

    // Check that the update was successful.
    if res.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    } // end if

    // Inform a user that they got approved for a bot usage.
    app_state
        .bot
        .send_message(
            ChatId(telegram_id),
            "Your subscription is <b>activated</b>!\n\nYou can use me right now!😍",
        )
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|error| {
            log::error!("{}", error.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    log::info!("Approved: {}", telegram_id);
    Ok(())
} // end fn approve
