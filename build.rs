use sqlx_migrate::{generate, DatabaseType};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=migrations");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    generate(
        root.join("migrations"),
        root.join("src/generated.rs"),
        DatabaseType::Sqlite,
    );
}



