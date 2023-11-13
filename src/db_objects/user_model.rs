// This file contains a database model of user settings.

use diesel::{
    query_builder::AsChangeset, ExpressionMethods, Identifiable, Insertable, PgConnection,
    QueryDsl, Queryable, RunQueryDsl,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{role_model::RoleId, status_model::StatusId};

/// This structure represents a user object in a database.
#[derive(
    Default, Clone, Debug, Queryable, Identifiable, Insertable, Serialize, Deserialize, AsChangeset,
)]
#[diesel(primary_key(telegram_id))]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    /// Unique user identifier in telegram.
    pub telegram_id: i64,
    /// Unique username of a user in telegram.
    pub telegram_username: Option<String>,
    /// Indicates whether or not a user is approved/banned/rejected etc.
    pub status_id: i32,
    /// API key from user's account on the exchange.
    pub exchange_api_key: Option<String>,
    /// API secret from user's account on the exchange.
    pub exchange_api_secret: Option<String>,
    /// Whether or not the bot was able to get the access to the user
    /// account on the exchange via provided API key and API secret.
    pub exchange_api_data_validated: bool,
    /// Deposit that is used to calculate the volume for a trade.
    /// (In USDT)
    pub deposit: Option<Decimal>,
    /// Deposit that is used to calculate the volume for a trade.
    /// (In %)
    pub risk: Option<Decimal>,
    /// Whether or not bot is allowed to trade for this user.
    pub bot_active: bool,
    /// The user role.
    pub role_id: i32,
} // end struct User

/// Implement functionality.
impl User {
    /// This function inserts a user into the database.
    /// The function returns an error if the user already exists.
    /// The function also returns an error if an insertion fails.
    pub fn insert_into_db(
        user: &Self,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let inserted_user = diesel::insert_into(crate::schema::users::table)
            .values(user.clone())
            .get_result(conn)?;

        Ok(inserted_user)
    } // end fn insert_into_db

    /// This function retrieves a user by their id from the database.
    /// The function returns an error if the operation fails.
    pub fn get_from_db_by_id(
        telegram_id: i64,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let retrieved_user: Self = crate::schema::users::table
            .filter(crate::schema::users::dsl::telegram_id.eq(telegram_id))
            .first::<Self>(conn)?;

        Ok(retrieved_user)
    } // end fn get_from_db_by_id

    /// This function updates the user settings.
    /// The function returns error if the operation fails.
    /// If the operation is successful, the function returns the updated user.
    pub fn update(user: &User, conn: &mut PgConnection) -> Result<Self, diesel::result::Error> {
        let updated_user: Self = diesel::update(user).set(user).get_result(conn)?;

        Ok(updated_user)
    } // end fn update

    /// This function sets a telegram username in the database to NULL.
    pub fn set_username_to_null(
        user: &User,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let updated_user: Self = diesel::update(user)
            .set(crate::schema::users::dsl::telegram_username.eq(Option::<String>::None))
            .get_result(conn)?;

        Ok(updated_user)
    } // end fn set_username_to_null

    /// This function retrieves an array of users with a particular status
    pub fn get_users_with_status(
        status_id: &StatusId,
        conn: &mut PgConnection,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let retrieved_users: Vec<Self> = crate::schema::users::dsl::users
            .filter(crate::schema::users::dsl::status_id.eq(i32::from(status_id.clone())))
            .load(conn)?;

        Ok(retrieved_users)
    } // end fn get_users_with_status

    /// This function retrieves an array of users with a particular role
    pub fn get_users_with_role(
        role_id: &RoleId,
        conn: &mut PgConnection,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let retrieved_users: Vec<Self> = crate::schema::users::dsl::users
            .filter(crate::schema::users::dsl::role_id.eq(i32::from(role_id.clone())))
            .load(conn)?;

        Ok(retrieved_users)
    } // end fn get_users_with_role
} // end impl User
