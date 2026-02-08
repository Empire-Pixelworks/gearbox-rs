pub trait Entity {
    type Id: Clone + Send + Sync;
    const COLLECTION: &'static str;
}