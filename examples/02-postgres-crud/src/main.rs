//! PostgreSQL Multi-Schema Example
//!
//! This example demonstrates:
//! - `#[derive(PgEntity)]` for basic CRUD operations
//! - `#[derive(Crud)]` for automatic REST endpoint generation
//! - `pg_queries!` for custom SQL queries
//! - `#[schema("name")]` for multi-schema support
//!
//! ```

use chrono::{DateTime, Utc};
use gearbox_rs::{
    Crud, IntoResponse, Json, PgClient, PgEntity, PgRepository, Path, gearbox_app, get, pg_queries,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// User Entity - in "users" schema
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, PgEntity, Crud)]
#[table("users")]
#[schema("users")]
#[crud(path = "/users")]
pub struct User {
    #[primary_key]
    #[auto_generated]
    pub id: Uuid,

    pub name: String,

    pub email: String,

    #[writeonly]
    pub password_hash: String,

    pub role: String,

    pub active: bool,

    #[readonly]
    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Post Entity - in "posts" schema
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, PgEntity, Crud)]
#[table("posts")]
#[schema("posts")]
#[crud(path = "/posts", read_only)]
pub struct Post {
    #[primary_key]
    #[auto_generated]
    pub id: Uuid,

    pub author_id: Uuid,

    pub title: String,

    pub content: String,

    pub published: bool,

    #[readonly]
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Custom Queries - each block declares its schema
// ============================================================================

// User queries target the "users" schema pool
pg_queries! {
    schema = "users";

    fn find_user_by_email(email: &str) -> Option<User> {
        "SELECT * FROM users WHERE email = $1"
    }

    fn find_active_users() -> Vec<User> {
        "SELECT * FROM users WHERE active = true ORDER BY name"
    }

    fn count_users_by_role(role: &str) -> i64 {
        "SELECT COUNT(*) FROM users WHERE role = $1"
    }

    fn deactivate_user(id: Uuid) -> bool {
        "UPDATE users SET active = false, updated_at = NOW() WHERE id = $1"
    }
}

// Post queries target the "posts" schema pool
pg_queries! {
    schema = "posts";

    fn find_posts_by_author(author_id: Uuid) -> Vec<Post> {
        "SELECT * FROM posts WHERE author_id = $1 ORDER BY created_at DESC"
    }

    fn publish_post(id: Uuid) -> bool {
        "UPDATE posts SET published = true WHERE id = $1"
    }

    fn delete_drafts_by_author(author_id: Uuid) -> u64 {
        "DELETE FROM posts WHERE author_id = $1 AND published = false"
    }
}

// ============================================================================
// Custom Routes
// ============================================================================

#[get("/users/by-email/{email}")]
async fn get_user_by_email(Path(email): Path<String>, db: Arc<PgClient>) -> impl IntoResponse {
    match db.find_user_by_email(&email).await {
        Ok(Some(user)) => {
            let response = UserResponse::from(user);
            (axum::http::StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => axum::http::StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[get("/stats/users")]
async fn get_user_stats(db: Arc<PgClient>) -> impl IntoResponse {
    let admin_count = db.count_users_by_role("admin").await.unwrap_or(0);
    let user_count = db.count_users_by_role("user").await.unwrap_or(0);
    let total: i64 = <PgClient as PgRepository<User>>::count(&*db)
        .await
        .unwrap_or(0);

    Json(serde_json::json!({
        "total": total,
        "by_role": {
            "admin": admin_count,
            "user": user_count
        }
    }))
}

#[get("/users/{id}/posts")]
async fn get_user_posts(Path(id): Path<Uuid>, db: Arc<PgClient>) -> impl IntoResponse {
    match db.find_posts_by_author(id).await {
        Ok(posts) => {
            let responses: Vec<PostResponse> = posts.into_iter().map(PostResponse::from).collect();
            Json(responses).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[gearbox_app]
fn main() {}
