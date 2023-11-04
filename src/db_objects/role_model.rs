// This file contains a database model of role model.

use diesel::{
    ExpressionMethods, Identifiable, Insertable, PgConnection, QueryDsl, Queryable, RunQueryDsl,
};
use serde::{Deserialize, Serialize};

/// This enumeration contains all user role titles.
#[derive(PartialEq, Clone)]
pub enum RoleId {
    User = 1,
    Admin,
    Superuser,
} // end enum RoleId

// Implement transformation from i32 to RoleId
impl From<i32> for RoleId {
    fn from(value: i32) -> Self {
        match value {
            1 => RoleId::User,
            2 => RoleId::Admin,
            3 => RoleId::Superuser,
            // Handle unexpected values appropriately, perhaps with a default case or an error
            _ => panic!("Invalid status id: {}", value),
        } // end match
    } // end from
} // end impl From<i32> for RoleId

/// Implement transformations of titles to role IDs.
impl From<RoleId> for i32 {
    fn from(value: RoleId) -> Self {
        match value {
            RoleId::User => 1,
            RoleId::Admin => 2,
            RoleId::Superuser => 3,
        } // end match
    } // end from
} // end impl From<RoleId> for i32

/// This structure represents a role object in a database.
#[derive(Default, Clone, Debug, Queryable, Identifiable, Insertable, Serialize, Deserialize)]
#[diesel(primary_key(role_id))]
#[diesel(table_name = crate::schema::role)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Role {
    /// Role ID.
    pub role_id: i32,
    /// Role title.
    pub title: String,
    /// Role description.
    pub role_description: String,
} // end struct Role

/// Implement functionality.
impl Role {
    /// This function inserts a role into the database.
    /// The function returns an error if the role already exists.
    /// The function also returns an error if an insertion fails.
    pub fn insert_into_db(
        role: &Self,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let inserted_role = diesel::insert_into(crate::schema::role::table)
            .values(role.clone())
            .get_result(conn)?;

        Ok(inserted_role)
    } // end fn insert_into_db

    /// This function retrieves a role by its id from the database.
    /// The function returns an error if the operation fails.
    pub fn get_from_db_by_id(
        role_id: i32,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let retrieved_role: Self = crate::schema::role::table
            .filter(crate::schema::role::dsl::role_id.eq(role_id))
            .first::<Self>(conn)?;

        Ok(retrieved_role)
    } // end fn get_from_db_by_id
} // end impl Role
