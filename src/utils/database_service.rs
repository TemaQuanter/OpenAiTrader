// This file contains a service with some useful features
// for working with databases.

use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};

/// This struct implements features to work with databases.
pub struct DatabaseService;

/// Implement functionality for the database service.
impl DatabaseService {
    /// This function a database connection pool.
    pub fn create_database_connection_pool() -> Pool<ConnectionManager<PgConnection>> {
        // create a new connection pool with the default config
        let manager = ConnectionManager::<PgConnection>::new(
            std::env::var("DATABASE_URL")
                .expect("Failed to find the environment variable DATABASE_URL"),
        ); // end config

        // Create a connection pool.
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create a pool of connections to a database");

        pool
    } // end fn create_app_state
    /// This function retrieves a new connection
    /// from a pool of connections to a database.
    pub fn get_pool_connection(
        pool: &Pool<ConnectionManager<PgConnection>>,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, diesel::r2d2::PoolError> {
        // Get a connection to the database.
        let conn = pool.get()?;

        Ok(conn)
    } // end fn get_pool_connection

    /// This function attempts to get a pooled database connection
    /// several times, but when all attempts are used, it returns an error.
    pub async fn try_get_pool_connection(
        pool: &Pool<ConnectionManager<PgConnection>>,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, diesel::r2d2::PoolError> {
        // Save the last error if the connection could not be established.
        let mut last_error: Option<diesel::r2d2::PoolError> = None;

        // Declare the number of attempts to make to establish the connection
        // with the database.
        let attempts = 3;

        // This loop is necessary to make several attempts to connect to the database
        // if an attempt fails at first.
        for attempt in 0..attempts {
            // Get a connection to the database.
            let conn = match DatabaseService::get_pool_connection(pool) {
                Ok(conn) => Some(conn),
                Err(error) => {
                    // There was an error while trying to access the database.
                    log::error!("{}", error);

                    // Save the latest error.
                    last_error = Some(error);

                    None
                } // end Err
            }; // end match

            // Check if it was possible to get a connection.
            if conn.is_some() {
                // the connection was established successfully.
                return Ok(conn.unwrap());
            } // end if

            // Check if it is not the last attempt.
            if attempt < attempts - 1 {
                // If the connection was not established, then wait for some time and repeat the attempt.
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            } // end if
        } // end for

        // If all attempts were unsuccessful, then return the last error.
        return Err(last_error.unwrap());
    } // end try_get_pool_connection

    /// This function is a wrapper on UniqueViolation error.
    /// It returns an object, if the result is Ok
    /// or throws Bool error
    ///     true - if UniqueViolation error occurred.
    ///     false - any other error occurred.
    pub fn _try_unwrap_unique_violation_error<Obj>(
        res: &Result<Obj, diesel::result::Error>,
    ) -> Result<Obj, bool>
    where
        Obj: Clone,
    {
        // Check if the result is Ok, or Error.
        match res {
            // The operation was successful, return an object.
            Ok(obj) => return Ok(obj.clone()),
            // Otherwise an error occurred.
            Err(error) => {
                // Check if the error was a DataBase error.
                // (Only database error occurs when UniqueViolation error occurs)
                if let diesel::result::Error::DatabaseError(kind, _) = &error {
                    // Check if the error is UniqueViolation error.
                    if let diesel::result::DatabaseErrorKind::UniqueViolation = kind {
                        // This is a UniqueViolation error.
                        return Err(true);
                    } // end if
                } // end if
            } // end Err
        } // end match

        // Otherwise it is some other error.
        Err(false)
    } // end fn try_unwrap_unique_violation_error
} // end impl DatabaseService
