//! Integration tests for gearbox-rs.
//!
//! These tests spin up an embedded PostgreSQL instance and make real HTTP requests.

use serde_json::{Value, json};

mod harness;
use harness::TestContext;

#[tokio::test]
async fn test_create_and_get_user() {
    let ctx = TestContext::setup().await;

    // Create a user
    let create_response = ctx
        .client
        .post(ctx.url("/users"))
        .json(&json!({
            "name": "John Doe",
            "email": "john@example.com",
            "password_hash": "hashed123",
            "role": "user",
            "active": true
        }))
        .send()
        .await
        .expect("Failed to create user");

    assert_eq!(create_response.status(), 201);

    let created: Value = create_response.json().await.unwrap();
    let user_id = created["id"].as_str().expect("id should be a string");

    // Get the user by ID
    let get_response = ctx
        .client
        .get(ctx.url(&format!("/users/{}", user_id)))
        .send()
        .await
        .expect("Failed to get user");

    assert_eq!(get_response.status(), 200);

    let user: Value = get_response.json().await.unwrap();
    assert_eq!(user["name"], "John Doe");
    assert_eq!(user["email"], "john@example.com");
    // password_hash should NOT be in response (writeonly)
    assert!(user.get("password_hash").is_none());
}

#[tokio::test]
async fn test_list_users_with_pagination() {
    let ctx = TestContext::setup().await;

    // Create multiple users
    for i in 0..5 {
        ctx.client
            .post(ctx.url("/users"))
            .json(&json!({
                "name": format!("User {}", i),
                "email": format!("user{}@example.com", i),
                "password_hash": "hash",
                "role": "user",
                "active": true
            }))
            .send()
            .await
            .expect("Failed to create user");
    }

    // List with pagination
    let response = ctx
        .client
        .get(ctx.url("/users?limit=2&offset=0"))
        .send()
        .await
        .expect("Failed to list users");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 5);
}

#[tokio::test]
async fn test_update_user() {
    let ctx = TestContext::setup().await;

    // Create a user
    let create_response = ctx
        .client
        .post(ctx.url("/users"))
        .json(&json!({
            "name": "Original Name",
            "email": "original@example.com",
            "password_hash": "hash",
            "role": "user",
            "active": true
        }))
        .send()
        .await
        .unwrap();

    let created: Value = create_response.json().await.unwrap();
    let user_id = created["id"].as_str().unwrap();

    // Patch update (partial)
    let patch_response = ctx
        .client
        .patch(ctx.url(&format!("/users/{}", user_id)))
        .json(&json!({
            "name": "Updated Name"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(patch_response.status(), 200);

    let updated: Value = patch_response.json().await.unwrap();
    assert_eq!(updated["name"], "Updated Name");
    assert_eq!(updated["email"], "original@example.com"); // unchanged
}

#[tokio::test]
async fn test_delete_user() {
    let ctx = TestContext::setup().await;

    // Create a user
    let create_response = ctx
        .client
        .post(ctx.url("/users"))
        .json(&json!({
            "name": "To Delete",
            "email": "delete@example.com",
            "password_hash": "hash",
            "role": "user",
            "active": true
        }))
        .send()
        .await
        .unwrap();

    let created: Value = create_response.json().await.unwrap();
    let user_id = created["id"].as_str().unwrap();

    // Delete
    let delete_response = ctx
        .client
        .delete(ctx.url(&format!("/users/{}", user_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(delete_response.status(), 204);

    // Verify deleted
    let get_response = ctx
        .client
        .get(ctx.url(&format!("/users/{}", user_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), 404);
}

#[tokio::test]
async fn test_custom_query_find_by_email() {
    let ctx = TestContext::setup().await;

    // Create a user
    ctx.client
        .post(ctx.url("/users"))
        .json(&json!({
            "name": "Test User",
            "email": "findme@example.com",
            "password_hash": "hash",
            "role": "admin",
            "active": true
        }))
        .send()
        .await
        .unwrap();

    // Use custom endpoint
    let response = ctx
        .client
        .get(ctx.url("/users/by-email/findme@example.com"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let user: Value = response.json().await.unwrap();
    assert_eq!(user["name"], "Test User");
}

#[tokio::test]
async fn test_user_stats() {
    let ctx = TestContext::setup().await;

    // Create users with different roles
    for (name, role) in [
        ("Admin 1", "admin"),
        ("Admin 2", "admin"),
        ("User 1", "user"),
    ] {
        ctx.client
            .post(ctx.url("/users"))
            .json(&json!({
                "name": name,
                "email": format!("{}@example.com", name.replace(' ', "").to_lowercase()),
                "password_hash": "hash",
                "role": role,
                "active": true
            }))
            .send()
            .await
            .unwrap();
    }

    let response = ctx
        .client
        .get(ctx.url("/stats/users"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let stats: Value = response.json().await.unwrap();
    assert_eq!(stats["total"], 3);
    assert_eq!(stats["by_role"]["admin"], 2);
    assert_eq!(stats["by_role"]["user"], 1);
}
