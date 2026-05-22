use openauth_core::context::create_auth_context;
use openauth_core::db::DbFieldType;
use openauth_core::options::OpenAuthOptions;
use openauth_scim::{scim, ScimOptions, UPSTREAM_PLUGIN_ID, VERSION};

#[test]
fn scim_public_constants_match_plugin_metadata() {
    let plugin = scim(ScimOptions::default());

    assert_eq!(UPSTREAM_PLUGIN_ID, "scim");
    assert_eq!(plugin.id, UPSTREAM_PLUGIN_ID);
    assert_eq!(plugin.version.as_deref(), Some(VERSION));
}

#[test]
fn scim_plugin_registers_snake_case_plural_schema() -> Result<(), Box<dyn std::error::Error>> {
    let context = create_auth_context(OpenAuthOptions {
        plugins: vec![scim(ScimOptions::default())],
        secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
        ..OpenAuthOptions::default()
    })?;

    let table = context
        .db_schema
        .table("scimProvider")
        .ok_or("missing scimProvider table")?;
    assert_eq!(table.name, "scim_providers");

    let provider_id = context.db_schema.field("scimProvider", "providerId")?;
    assert_eq!(provider_id.name, "provider_id");
    assert_eq!(provider_id.field_type, DbFieldType::String);
    assert!(provider_id.required);
    assert!(provider_id.unique);

    let scim_token = context.db_schema.field("scimProvider", "scimToken")?;
    assert_eq!(scim_token.name, "scim_token");
    assert_eq!(scim_token.field_type, DbFieldType::String);
    assert!(scim_token.required);
    assert!(scim_token.unique);
    assert!(!scim_token.returned);

    let organization_id = context.db_schema.field("scimProvider", "organizationId")?;
    assert_eq!(organization_id.name, "organization_id");
    assert_eq!(organization_id.field_type, DbFieldType::String);
    assert!(!organization_id.required);
    assert!(organization_id.index);

    let user_id = context.db_schema.field("scimProvider", "userId")?;
    assert_eq!(user_id.name, "user_id");
    assert_eq!(user_id.field_type, DbFieldType::String);
    assert!(!user_id.required);
    assert!(user_id.index);
    assert!(user_id.foreign_key.is_some());

    let user_profile = context
        .db_schema
        .table("scimUserProfile")
        .ok_or("missing scimUserProfile table")?;
    assert_eq!(user_profile.name, "scim_user_profiles");
    assert_eq!(
        context
            .db_schema
            .field("scimUserProfile", "attributes")?
            .field_type,
        DbFieldType::Json
    );

    let group_profile = context
        .db_schema
        .table("scimGroupProfile")
        .ok_or("missing scimGroupProfile table")?;
    assert_eq!(group_profile.name, "scim_group_profiles");
    assert_eq!(
        context
            .db_schema
            .field("scimGroupProfile", "teamId")?
            .field_type,
        DbFieldType::String
    );

    Ok(())
}

#[test]
fn scim_plugin_registers_expected_endpoint_surface() {
    let plugin = scim(ScimOptions::default());
    let endpoints = plugin
        .endpoints
        .iter()
        .map(|endpoint| (endpoint.method.clone(), endpoint.path.as_str()))
        .collect::<Vec<_>>();

    assert!(endpoints.contains(&(http::Method::POST, "/scim/generate-token")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/list-provider-connections")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/get-provider-connection")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/delete-provider-connection")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/Users")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Users")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Users/:userId")));
    assert!(endpoints.contains(&(http::Method::PUT, "/scim/v2/Users/:userId")));
    assert!(endpoints.contains(&(http::Method::PATCH, "/scim/v2/Users/:userId")));
    assert!(endpoints.contains(&(http::Method::DELETE, "/scim/v2/Users/:userId")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/Users/.search")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Groups")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/Groups")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Groups/:groupId")));
    assert!(endpoints.contains(&(http::Method::PUT, "/scim/v2/Groups/:groupId")));
    assert!(endpoints.contains(&(http::Method::PATCH, "/scim/v2/Groups/:groupId")));
    assert!(endpoints.contains(&(http::Method::DELETE, "/scim/v2/Groups/:groupId")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/Groups/.search")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/.search")));
    assert!(endpoints.contains(&(http::Method::POST, "/scim/v2/Bulk")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Me")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/ServiceProviderConfig")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Schemas")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/Schemas/:schemaId")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/ResourceTypes")));
    assert!(endpoints.contains(&(http::Method::GET, "/scim/v2/ResourceTypes/:resourceTypeId")));
}

#[test]
fn scim_plugin_registers_endpoint_media_types_and_openapi_metadata() {
    let plugin = scim(ScimOptions::default());
    let expected_operation_ids = [
        "generateSCIMToken",
        "listSCIMProviderConnections",
        "getSCIMProviderConnection",
        "deleteSCIMProviderConnection",
        "createSCIMUser",
        "listSCIMUsers",
        "getSCIMUser",
        "updateSCIMUser",
        "patchSCIMUser",
        "deleteSCIMUser",
        "searchSCIMUsers",
        "createSCIMGroup",
        "listSCIMGroups",
        "getSCIMGroup",
        "updateSCIMGroup",
        "patchSCIMGroup",
        "deleteSCIMGroup",
        "searchSCIMGroups",
        "searchSCIMResources",
        "bulkSCIM",
        "getSCIMMe",
        "getSCIMServiceProviderConfig",
        "getSCIMSchemas",
        "getSCIMSchema",
        "getSCIMResourceTypes",
        "getSCIMResourceType",
    ];
    let operation_ids = plugin
        .endpoints
        .iter()
        .map(|endpoint| endpoint.options.operation_id.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(
        operation_ids,
        expected_operation_ids
            .iter()
            .map(|operation_id| Some(*operation_id))
            .collect::<Vec<_>>()
    );

    for endpoint in &plugin.endpoints {
        assert!(
            endpoint.options.operation_id.is_some(),
            "{} {} should have an operation id",
            endpoint.method,
            endpoint.path
        );
        assert!(
            endpoint.options.openapi.is_some(),
            "{} {} should have OpenAPI metadata",
            endpoint.method,
            endpoint.path
        );
    }

    let create_user = plugin
        .endpoints
        .iter()
        .find(|endpoint| endpoint.method == http::Method::POST && endpoint.path == "/scim/v2/Users")
        .expect("create SCIM user endpoint should exist");
    assert_eq!(
        create_user.options.allowed_media_types,
        vec!["application/scim+json", "application/json"]
    );

    let metadata = plugin
        .endpoints
        .iter()
        .find(|endpoint| {
            endpoint.method == http::Method::GET
                && endpoint.path == "/scim/v2/ServiceProviderConfig"
        })
        .expect("metadata endpoint should exist");
    assert!(metadata.options.allowed_media_types.is_empty());
}
