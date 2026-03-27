use gearbox_rs_macros::cog;
use std::sync::Arc;

fn some_fn() -> String {
    String::new()
}

#[cog]
struct BadCog {
    #[inject]
    #[default(some_fn)]
    field: Arc<String>,
}

fn main() {}
