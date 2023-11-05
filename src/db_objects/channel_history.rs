// This file contains a database model of channel history model.

use chrono::{DateTime, Utc};
use diesel::{
    ExpressionMethods, Identifiable, Insertable, PgConnection, QueryDsl, Queryable, RunQueryDsl,
};
use serde::{Deserialize, Serialize};

/// This structure represents a channel history object in a database.
#[derive(Default, Clone, Debug, Queryable, Identifiable, Insertable, Serialize, Deserialize)]
#[diesel(primary_key(message_id))]
#[diesel(table_name = crate::schema::channel_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChannelHistory {
    /// Message ID.
    pub message_id: i32,
    /// The ID of the message the current message replies to.
    pub response_message_id: Option<i32>,
    /// Message text.
    pub message_text: Option<String>,
    /// The timestamp which shows the time the message was received by the program.
    pub message_timestamp: DateTime<Utc>,
} // end struct ChannelHistory

/// Implement functionality.
impl ChannelHistory {
    /// This function inserts a message into the database.
    /// The function returns an error if the message already exists.
    /// The function also returns an error if an insertion fails.
    pub fn insert_into_db(
        message: &Self,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let inserted_message = diesel::insert_into(crate::schema::channel_history::table)
            .values(message.clone())
            .get_result(conn)?;

        Ok(inserted_message)
    } // end fn insert_into_db

    /// This function retrieves a message by its id from the database.
    /// The function returns an error if the operation fails.
    pub fn get_from_db_by_id(
        message_id: i32,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let retrieved_message: Self = crate::schema::channel_history::table
            .filter(crate::schema::channel_history::dsl::message_id.eq(message_id))
            .first::<Self>(conn)?;

        Ok(retrieved_message)
    } // end fn get_from_db_by_id

    /// This function removes all messages that are older than a particular
    /// timestamp. (This function is necessary not to overload the database
    /// with messages).s
    pub fn remove_old_messages(
        timestamp: DateTime<Utc>,
        conn: &mut PgConnection,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(crate::schema::channel_history::table)
            .filter(crate::schema::channel_history::dsl::message_timestamp.le(timestamp))
            .execute(conn)?;

        Ok(())
    } // end fn remove_old_messages
} // end impl ChannelHistory
