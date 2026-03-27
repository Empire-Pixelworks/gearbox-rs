use gearbox_rs_macros::PgEntity;

#[derive(PgEntity)]
#[table("things")]
enum NotAStruct {
    A,
    B,
}

fn main() {}
