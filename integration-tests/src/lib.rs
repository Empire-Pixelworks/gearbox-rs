//! Integration Tests Library
//!
//! This library provides entities, routes, and queries used for integration testing
//! the gearbox-rs framework with real PostgreSQL and HTTP requests.
//!
//! Features tested:
//! - `#[derive(PgEntity)]` for basic CRUD operations
//! - `#[derive(Crud)]` for automatic REST endpoint generation
//! - `pg_queries!` for custom SQL queries

use chrono::{DateTime, Utc};
use gearbox_rs::{
    Crud, IntoResponse, Json, PgClient, PgEntity, PgRepository, Path, get, pg_queries,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// User Entity - Full CRUD with REST endpoints
// ============================================================================

/// User entity with automatic REST API generation.
///
/// The `#[derive(Crud)]` macro generates:
/// - `UserCreate` - DTO for POST /users
/// - `UserUpdate` - DTO for PATCH /users/{id}
/// - `UserQuery` - Query params for GET /users
/// - `UserResponse` - DTO for responses (excludes password_hash)
/// - Route handlers for all CRUD operations
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, PgEntity, Crud)]
#[table("users")]
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
// Post Entity - Read-only REST endpoints (no create/update/delete via API)
// ============================================================================

/// Post entity with read-only REST API.
///
/// Using `read_only` means only GET endpoints are generated.
/// Posts are created through custom routes or internal logic.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, PgEntity, Crud)]
#[table("posts")]
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
// Custom Queries with pg_queries!
// ============================================================================

/// Custom query result for user with post count
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserWithPostCount {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub post_count: i64,
}

/// Custom query result for post with author info
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PostWithAuthor {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
    pub author_name: String,
    pub author_email: String,
}

// Define custom SQL queries
pg_queries! {
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

    fn get_users_with_post_counts() -> Vec<UserWithPostCount> {
        "SELECT u.id, u.name, u.email, COUNT(p.id) as post_count
         FROM users u
         LEFT JOIN posts p ON u.id = p.author_id
         GROUP BY u.id, u.name, u.email
         ORDER BY post_count DESC"
    }

    fn get_published_posts_with_authors() -> Vec<PostWithAuthor> {
        "SELECT p.id, p.title, p.content, p.published, p.created_at,
                u.name as author_name, u.email as author_email
         FROM posts p
         JOIN users u ON p.author_id = u.id
         WHERE p.published = true
         ORDER BY p.created_at DESC"
    }

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
// Custom Routes (using the generated DTOs and custom queries)
// ============================================================================

#[get("/users/by-email/{email}")]
pub async fn get_user_by_email(Path(email): Path<String>, db: Arc<PgClient>) -> impl IntoResponse {
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

#[get("/users/with-post-counts")]
pub async fn get_users_with_counts(db: Arc<PgClient>) -> impl IntoResponse {
    match db.get_users_with_post_counts().await {
        Ok(users) => Json(users).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[get("/stats/users")]
pub async fn get_user_stats(db: Arc<PgClient>) -> impl IntoResponse {
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

#[get("/posts/published")]
pub async fn get_published_posts(db: Arc<PgClient>) -> impl IntoResponse {
    match db.get_published_posts_with_authors().await {
        Ok(posts) => Json(posts).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[get("/users/{id}/posts")]
pub async fn get_user_posts(Path(id): Path<Uuid>, db: Arc<PgClient>) -> impl IntoResponse {
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
