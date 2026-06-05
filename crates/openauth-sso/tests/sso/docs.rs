#[test]
fn sso_readme_documents_upstream_parity() {
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let contents = std::fs::read_to_string(readme).expect("openauth-sso README");
    assert!(
        contents.contains("## Upstream parity (Better Auth 1.6.9)"),
        "expected upstream parity section in openauth-sso README"
    );
}
