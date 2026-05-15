use std::sync::Arc;

use http::{Method, StatusCode};
use openauth_core::db::MemoryAdapter;
use openauth_plugins::organization::{DynamicAccessControlOptions, OrganizationOptions};
use serde_json::json;

#[tokio::test]
async fn dynamic_access_control_crud_roles_and_rejects_assigned_delete(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(MemoryAdapter::new());
    let options = OrganizationOptions::builder()
        .dynamic_access_control(DynamicAccessControlOptions {
            enabled: true,
            maximum_roles_per_organization: Some(3),
        })
        .build();
    let auth = super::test_router(adapter, options)?;

    let ada = super::sign_up(&auth, "Ada", "ada-dac@example.com").await?;
    let org = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create",
        json!({"name":"Acme DAC","slug":"acme-dac"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(org.status, StatusCode::OK);

    let role = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create-role",
        json!({
            "role": "billing",
            "permission": { "organization": ["update"], "ac": ["read"] }
        }),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(role.status, StatusCode::OK);
    assert_eq!(role.body["role"], "billing");
    let role_id = role.body["id"].as_str().ok_or("missing role id")?;

    let listed = super::request_json(
        &auth,
        Method::GET,
        "/api/auth/organization/list-roles",
        json!({}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.body.as_array().map(Vec::len), Some(1));

    let updated = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/update-role",
        json!({
            "roleId": role_id,
            "permission": { "organization": ["update"], "invitation": ["create"], "ac": ["read"] }
        }),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(updated.status, StatusCode::OK);

    let ben = super::sign_up(&auth, "Ben", "ben-dac@example.com").await?;
    let member = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/add-member",
        json!({"userId": ben.user_id, "role": "billing"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(member.status, StatusCode::OK);

    let active = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/set-active",
        json!({"organizationId": org.body["id"]}),
        Some(&ben.cookie),
    )
    .await?;
    assert_eq!(active.status, StatusCode::OK);
    let permission = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/has-permission",
        json!({"permissions": {"invitation": ["create"]}}),
        Some(&ben.cookie),
    )
    .await?;
    assert_eq!(permission.status, StatusCode::OK);
    assert_eq!(permission.body["success"], true);

    let assigned_delete = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/delete-role",
        json!({"roleId": role_id}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(assigned_delete.status, StatusCode::BAD_REQUEST);
    assert_eq!(assigned_delete.body["code"], "ROLE_IS_ASSIGNED_TO_MEMBERS");

    Ok(())
}

#[tokio::test]
async fn dynamic_access_control_rejects_invalid_resources_limits_and_cross_org_role_ids(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(MemoryAdapter::new());
    let options = OrganizationOptions::builder()
        .dynamic_access_control(DynamicAccessControlOptions {
            enabled: true,
            maximum_roles_per_organization: Some(1),
        })
        .build();
    let auth = super::test_router(adapter, options)?;

    let ada = super::sign_up(&auth, "Ada", "ada-dac-hardening@example.com").await?;
    let first = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create",
        json!({"name":"First DAC","slug":"first-dac"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(first.status, StatusCode::OK);
    let first_id = first.body["id"].as_str().ok_or("missing first org id")?;

    let invalid = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create-role",
        json!({
            "organizationId": first_id,
            "role": "invalid",
            "permission": { "billing": ["read"] }
        }),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.body["code"], "INVALID_RESOURCE");

    let role = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create-role",
        json!({
            "organizationId": first_id,
            "role": "ops",
            "permission": { "organization": ["update"], "ac": ["read"] }
        }),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(role.status, StatusCode::OK);
    let role_id = role.body["id"].as_str().ok_or("missing role id")?;

    let too_many = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create-role",
        json!({
            "organizationId": first_id,
            "role": "finance",
            "permission": { "organization": ["update"] }
        }),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(too_many.status, StatusCode::BAD_REQUEST);
    assert_eq!(too_many.body["code"], "TOO_MANY_ROLES");

    let second = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create",
        json!({"name":"Second DAC","slug":"second-dac"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(second.status, StatusCode::OK);
    let second_id = second.body["id"].as_str().ok_or("missing second org id")?;

    let cross_org = super::request_json(
        &auth,
        Method::GET,
        &format!("/api/auth/organization/get-role?organizationId={second_id}&roleId={role_id}"),
        json!({}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(cross_org.status, StatusCode::BAD_REQUEST);
    assert_eq!(cross_org.body["code"], "ROLE_NOT_FOUND");

    Ok(())
}
