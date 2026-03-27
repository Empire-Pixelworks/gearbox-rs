//! Tests for the pg_queries! macro.
//!
//! These tests verify the macro generates correct trait and impl.

use gearbox_rs_macros::pg_queries;

// Test struct that would normally derive FromRow
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for User {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(User {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            active: row.try_get("active")?,
        })
    }
}

// Custom return struct
#[derive(Debug, Clone)]
pub struct UserSummary {
    pub id: String,
    pub name: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for UserSummary {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(UserSummary {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
        })
    }
}

// Generate the PgQueries trait and impl
pg_queries! {
    // Option<T> - fetch_optional
    fn find_user_by_id(id: &str) -> Option<User> {
        "SELECT * FROM users WHERE id = $1"
    }

    // Vec<T> - fetch_all
    fn find_users_by_status(status: &str) -> Vec<User> {
        "SELECT * FROM users WHERE status = $1"
    }

    // Vec<T> with multiple params
    fn find_users_by_status_and_role(status: &str, role: &str) -> Vec<User> {
        "SELECT * FROM users WHERE status = $1 AND role = $2"
    }

    // Custom return type
    fn get_user_summary(id: &str) -> Option<UserSummary> {
        "SELECT id, name FROM users WHERE id = $1"
    }

    // Single entity (fetch_one)
    fn get_user_or_fail(id: &str) -> User {
        "SELECT * FROM users WHERE id = $1"
    }

    // Scalar i64
    fn count_users() -> i64 {
        "SELECT COUNT(*) FROM users"
    }

    // Scalar with param
    fn count_users_by_status(status: &str) -> i64 {
        "SELECT COUNT(*) FROM users WHERE status = $1"
    }

    // Bool return (rows_affected > 0)
    fn deactivate_user(id: &str) -> bool {
        "UPDATE users SET active = false WHERE id = $1"
    }

    // u64 return (rows_affected)
    fn delete_inactive_users() -> u64 {
        "DELETE FROM users WHERE active = false"
    }

    // Unit return (execute)
    fn log_user_action(user_id: &str, action: &str) {
        "INSERT INTO audit_log (user_id, action, created_at) VALUES ($1, $2, NOW())"
    }

    // No params
    fn find_all_users() -> Vec<User> {
        "SELECT * FROM users"
    }
}

// Verify the trait was generated
#[test]
fn test_trait_generated() {
    // This test passes if PgQueries trait exists
    fn _assert_trait_exists<T: PgQueries>() {}
}

// Verify impl exists for PgClient
#[test]
fn test_impl_for_pg_client() {
    fn _assert_impl() {
        fn _check<T: PgQueries>(_: &T) {}
        fn _use_it(client: &gearbox_rs_postgres::PgClient) {
            _check(client);
        }
    }
}

// Verify method signatures by checking they're callable (compile-time check)
#[test]
fn test_method_signatures_compile() {
    // This function would use the methods if we had a real client
    #[allow(dead_code)]
    async fn _use_methods(client: &gearbox_rs_postgres::PgClient) {
        use crate::PgQueries;

        let _: Result<Option<User>, _> = client.find_user_by_id("123").await;
        let _: Result<Vec<User>, _> = client.find_users_by_status("active").await;
        let _: Result<Vec<User>, _> = client
            .find_users_by_status_and_role("active", "admin")
            .await;
        let _: Result<Option<UserSummary>, _> = client.get_user_summary("123").await;
        let _: Result<User, _> = client.get_user_or_fail("123").await;
        let _: Result<i64, _> = client.count_users().await;
        let _: Result<i64, _> = client.count_users_by_status("active").await;
        let _: Result<bool, _> = client.deactivate_user("123").await;
        let _: Result<u64, _> = client.delete_inactive_users().await;
        let _: Result<(), _> = client.log_user_action("123", "login").await;
        let _: Result<Vec<User>, _> = client.find_all_users().await;
    }
}

#[test]
fn test_no_params_query() {
    #[allow(dead_code)]
    async fn _use_it(client: &gearbox_rs_postgres::PgClient) {
        use crate::PgQueries;
        let _ = client.find_all_users().await;
        let _ = client.count_users().await;
    }
}

#[test]
fn test_multiple_params_query() {
    #[allow(dead_code)]
    async fn _use_it(client: &gearbox_rs_postgres::PgClient) {
        use crate::PgQueries;
        let _ = client
            .find_users_by_status_and_role("active", "admin")
            .await;
        let _ = client.log_user_action("user_123", "logged_in").await;
    }
}
