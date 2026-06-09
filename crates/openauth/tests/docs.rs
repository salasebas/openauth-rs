#![allow(clippy::expect_used)]

#[test]
fn readme_sql_adapter_example_uses_umbrella_sqlx_reexport() {
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let contents = std::fs::read_to_string(readme).expect("openauth README");

    assert!(
        contents.contains("openauth::sqlx::SqliteAdapter"),
        "expected umbrella SQLx adapter import in openauth README"
    );
    assert!(
        !contents.contains("use openauth_sqlx::"),
        "openauth README should not bypass the umbrella crate with direct openauth_sqlx imports"
    );
    assert!(
        contents.contains("features = [\"sqlx-sqlite\"]"),
        "expected sqlx-sqlite feature documented alongside the adapter example"
    );
    assert!(
        contents.contains("openauth::prelude"),
        "expected prelude import in openauth README quick start"
    );
    assert!(
        contents.contains(".build()\n        .await?"),
        "expected async build in openauth README quick start"
    );
}
