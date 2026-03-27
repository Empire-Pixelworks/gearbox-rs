use gearbox_rs_macros::cog;

#[cog]
struct BadCog {
    #[inject]
    field: String,
}

fn main() {}
