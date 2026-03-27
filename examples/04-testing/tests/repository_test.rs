//! Demonstrates testing with a pre-populated "database" stub.
//!
//! Instead of connecting to a real database, build an ItemRepository
//! with test data and register it via hub.registry_put().

use gearbox_rs::TestHubBuilder;
use testing_example::repository::{Item, ItemRepository};

#[tokio::test]
async fn test_repository_with_stub_data() {
    let repo = ItemRepository::with_items(vec![
        Item {
            id: "1".into(),
            name: "Widget".into(),
        },
        Item {
            id: "2".into(),
            name: "Gadget".into(),
        },
    ]);

    let hub = TestHubBuilder::new().build();
    hub.registry_put(repo).unwrap();

    let repo = hub.registry_get::<ItemRepository>().unwrap();
    assert_eq!(repo.get("1").unwrap().name, "Widget");
    assert_eq!(repo.get("2").unwrap().name, "Gadget");
    assert!(repo.get("999").is_none());
}

#[tokio::test]
async fn test_empty_repository() {
    let hub = TestHubBuilder::new().build();
    hub.registry_put(ItemRepository::with_items(vec![]))
        .unwrap();

    let repo = hub.registry_get::<ItemRepository>().unwrap();
    assert!(repo.list().is_empty());
    assert!(repo.get("1").is_none());
}
