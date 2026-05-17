use http::{Method, StatusCode};
use openauth_core::db::{DbAdapter, DbValue, FindOne, Where};
use openauth_passkey::PasskeyOptions;

use crate::support::{
    json_request, seed_passkey, seed_user_two, seeded_router, session_cookie_for,
};

#[tokio::test]
async fn update_and_delete_require_passkey_ownership() -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;
    seed_user_two(adapter.as_ref()).await?;
    seed_passkey(
        adapter.as_ref(),
        "passkey_1",
        "user_1",
        "original",
        "credential-id",
    )
    .await?;
    let user_two_cookie = session_cookie_for(&adapter, "user_2", "token_2").await?;

    let update = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/update-passkey",
            r#"{"id":"passkey_1","name":"hacked"}"#,
            Some(&user_two_cookie),
        )?)
        .await?;
    assert_eq!(update.status(), StatusCode::UNAUTHORIZED);

    let delete = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/delete-passkey",
            r#"{"id":"passkey_1"}"#,
            Some(&user_two_cookie),
        )?)
        .await?;
    assert_eq!(delete.status(), StatusCode::UNAUTHORIZED);

    let unchanged = adapter
        .find_one(
            FindOne::new("passkey")
                .where_clause(Where::new("id", DbValue::String("passkey_1".to_owned()))),
        )
        .await?
        .ok_or("missing passkey")?;
    assert_eq!(
        unchanged.get("name"),
        Some(&DbValue::String("original".to_owned()))
    );
    Ok(())
}
