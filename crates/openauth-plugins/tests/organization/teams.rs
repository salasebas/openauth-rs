use std::sync::Arc;

use http::{Method, StatusCode};
use openauth_core::db::MemoryAdapter;
use openauth_plugins::organization::{OrganizationOptions, TeamOptions};
use serde_json::json;

#[tokio::test]
async fn team_routes_cover_default_team_members_and_active_team(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(MemoryAdapter::new());
    let options = OrganizationOptions::builder()
        .teams(TeamOptions {
            enabled: true,
            create_default_team: true,
            maximum_teams: Some(3),
            maximum_members_per_team: Some(3),
            allow_removing_all_teams: false,
        })
        .build();
    let auth = super::test_router(adapter, options)?;

    let ada = super::sign_up(&auth, "Ada", "ada-team@example.com").await?;
    let org = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create",
        json!({"name":"Acme Teams","slug":"acme-teams"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(org.status, StatusCode::OK);

    let full = super::request_json(
        &auth,
        Method::GET,
        "/api/auth/organization/get-full-organization",
        json!({}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(full.status, StatusCode::OK);
    assert_eq!(full.body["teams"].as_array().map(Vec::len), Some(1));

    let team = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/create-team",
        json!({"name":"Engineering"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(team.status, StatusCode::OK);
    let team_id = team.body["id"].as_str().ok_or("missing team id")?;

    let ben = super::sign_up(&auth, "Ben", "ben-team@example.com").await?;
    let member = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/add-member",
        json!({"userId": ben.user_id, "role": "member"}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(member.status, StatusCode::OK);

    let team_member = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/add-team-member",
        json!({"teamId": team_id, "userId": ben.user_id}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(team_member.status, StatusCode::OK);

    let listed = super::request_json(
        &auth,
        Method::GET,
        &format!("/api/auth/organization/list-team-members?teamId={team_id}"),
        json!({}),
        Some(&ada.cookie),
    )
    .await?;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.body.as_array().map(Vec::len), Some(2));

    let active = super::request_json(
        &auth,
        Method::POST,
        "/api/auth/organization/set-active-team",
        json!({"teamId": team_id}),
        Some(&ben.cookie),
    )
    .await?;
    assert_eq!(active.status, StatusCode::OK);

    Ok(())
}
