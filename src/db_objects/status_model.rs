// This file contains a database model of status model.

use diesel::{
    ExpressionMethods, Identifiable, Insertable, PgConnection, QueryDsl, Queryable, RunQueryDsl,
};
use serde::{Deserialize, Serialize};

/// This enumeration contains all user status titles.
pub enum StatusId {
    Pending = 1,
    Rejected,
    Approved,
    Banned,
} // end enum StatusId

// Implement transformation from i32 to StatusId
impl From<i32> for StatusId {
    fn from(value: i32) -> Self {
        match value {
            1 => StatusId::Pending,
            2 => StatusId::Rejected,
            3 => StatusId::Approved,
            4 => StatusId::Banned,
            // Handle unexpected values appropriately, perhaps with a default case or an error
            _ => panic!("Invalid status id: {}", value),
        } // end match
    } // end from
} // end impl From<i32> for StatusId

/// Implement transformations of titles to status IDs.
impl From<StatusId> for i32 {
    fn from(value: StatusId) -> Self {
        match value {
            StatusId::Pending => 1,
            StatusId::Rejected => 2,
            StatusId::Approved => 3,
            StatusId::Banned => 4,
        } // end match
    } // end from
} // end impl From<StatusId> for i32

/// This structure represents a status object in a database.
#[derive(Default, Clone, Debug, Queryable, Identifiable, Insertable, Serialize, Deserialize)]
#[diesel(primary_key(status_id))]
#[diesel(table_name = crate::schema::status)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Status {
    /// Status ID.
    #[diesel(deserialize_as = i32)]
    pub status_id: Option<i32>,
    /// Status title.
    pub title: String,
    /// Status description.
    pub status_description: String,
} // end struct Status

/// Implement functionality.
impl Status {
    /// This function inserts a status into the database.
    /// The function returns an error if the status already exists.
    /// The function also returns an error if an insertion fails.
    pub fn insert_into_db(
        status: &Self,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let inserted_status = diesel::insert_into(crate::schema::status::table)
            .values(status.clone())
            .get_result(conn)?;

        Ok(inserted_status)
    } // end fn insert_into_db

    /// This function retrieves a status by its id from the database.
    /// The function returns an error if the operation fails.
    pub fn get_from_db_by_id(
        status_id: i32,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let retrieved_status: Self = crate::schema::status::table
            .filter(crate::schema::status::dsl::status_id.eq(status_id))
            .first::<Self>(conn)?;

        Ok(retrieved_status)
    } // end fn get_from_db_by_id
} // end impl Status
