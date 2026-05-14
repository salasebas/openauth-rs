use std::sync::{Arc, Mutex};

use http::{Method, StatusCode};
use openauth_core::db::MemoryAdapter;
use openauth_plugins::organization::OrganizationOptions;
use serde_json::json;

#[tokio::test]
async fn invitation_email_hook_runs_inline() -> Result<(), Box<dyn std::error::Error>> {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let captured = sent.clone();
    let options = OrganizationOptions::builder()
        .send_invitation_email(Arc::new(move |email| {
            captured
                .lock()
                .map_err(|error| openauth_core::error::OpenAuthError::Api(error.to_string()))?
                .push((
                    email.email.clone(),
                    email.role.clone(),
                    email.organization.id.clone(),
                ));
            Ok(())
        }))
        .build();
    let auth = super::test_router(Arc::new(MemoryAdapter::new()), options)?;

    let ada = super::sign_up(&auth, "Ada", "ada-hook@example.com").await?;
    let org = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create",
        json!({"name":"Acme Hooks","slug":"acme-hooks"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(org.status, StatusCode::OK);

    let invite = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/invite-member",
        json!({"email":"invited-hook@example.com","role":"member"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(invite.status, StatusCode::OK);

    let sent = sent.lock().map_err(|error| error.to_string())?;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "invited-hook@example.com");
    assert_eq!(sent[0].1, "member");
    assert_eq!(sent[0].2, org.body["id"]);
    Ok(())
}
