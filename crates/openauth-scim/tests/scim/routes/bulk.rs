use super::*;

#[tokio::test]
async fn bulk_route_executes_get_user_operations() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);
    let created = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Users",
            r#"{"userName":"bulk@example.com"}"#,
            Some(&token),
        ))
        .await
        .expect("request should succeed");
    let user_id = json_body(created)["id"].as_str().expect("id").to_owned();

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &format!(
                r#"{{
                    "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                    "Operations":[{{"method":"GET","path":"/Users/{user_id}"}}]
                }}"#
            ),
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(
        body["schemas"][0],
        "urn:ietf:params:scim:api:messages:2.0:BulkResponse"
    );
    assert_eq!(body["Operations"][0]["status"]["code"], 200);
    assert!(body["Operations"][0]["location"]
        .as_str()
        .expect("location")
        .ends_with(&format!("/scim/v2/Users/{user_id}")));
    assert!(body["Operations"][0]["version"].as_str().is_some());
    assert_eq!(
        body["Operations"][0]["response"]["userName"],
        "bulk@example.com"
    );
}

#[tokio::test]
async fn bulk_route_requires_bulk_id_for_post_and_respects_fail_on_errors() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            r#"{
                "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                "failOnErrors":1,
                "Operations":[
                    {"method":"POST","path":"/Users","data":{"userName":"missing-bulkid@example.com"}},
                    {"method":"GET","path":"/Users/never-runs"}
                ]
            }"#,
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"].as_array().expect("ops").len(), 1);
    assert_eq!(body["Operations"][0]["status"]["code"], 400);
    assert_eq!(
        body["Operations"][0]["response"]["scimType"],
        "invalidValue"
    );
}

#[tokio::test]
async fn bulk_route_resolves_bulk_id_for_user_group_membership() {
    let (adapter, router, context) =
        router_with_context_and_organization(ScimOptions::default()).expect("router");
    let (owner_cookie, owner_id) =
        session_cookie_with_user(adapter.as_ref(), &context, "bulk-owner@example.com")
            .await
            .expect("owner session");
    seed_organization(adapter.as_ref(), "org_1")
        .await
        .expect("org");
    seed_member(adapter.as_ref(), "org_1", &owner_id, "owner")
        .await
        .expect("owner member");
    let token = generate_scim_token(&router, &owner_cookie, "okta", Some("org_1")).await;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            r#"{
                "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                "Operations":[
                    {
                        "method":"POST",
                        "path":"/Users",
                        "bulkId":"user-1",
                        "data":{"userName":"bulk-created@example.com","name":{"formatted":"Bulk Created"}}
                    },
                    {
                        "method":"POST",
                        "path":"/Groups",
                        "bulkId":"group-1",
                        "data":{"displayName":"Bulk Team","members":[{"value":"bulkId:user-1"}]}
                    },
                    {"method":"GET","path":"bulkId:group-1"}
                ]
            }"#,
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"][0]["status"]["code"], 201);
    assert_eq!(body["Operations"][1]["status"]["code"], 201);
    assert_eq!(body["Operations"][2]["status"]["code"], 200);
    assert_eq!(
        body["Operations"][2]["response"]["members"][0]["display"],
        "Bulk Created"
    );
}

#[tokio::test]
async fn bulk_route_executes_put_patch_and_delete_operations() {
    let (adapter, router, context) =
        router_with_context_and_organization(ScimOptions::default()).expect("router");
    let (owner_cookie, owner_id) =
        session_cookie_with_user(adapter.as_ref(), &context, "bulk-mutate-owner@example.com")
            .await
            .expect("owner session");
    seed_organization(adapter.as_ref(), "org_1")
        .await
        .expect("org");
    seed_member(adapter.as_ref(), "org_1", &owner_id, "owner")
        .await
        .expect("owner member");
    let token = generate_scim_token(&router, &owner_cookie, "okta", Some("org_1")).await;
    let user_id = create_scim_user(&router, &token, "bulk-put@example.com", "Bulk Put").await;
    let group_id = create_scim_group(&router, &token, "Bulk Patch Team", "bulk-patch", &[]).await;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &format!(
                r#"{{
                    "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                    "Operations":[
                        {{
                            "method":"PUT",
                            "path":"/Users/{user_id}",
                            "data":{{"userName":"bulk-put-updated@example.com","title":"Updated"}}
                        }},
                        {{
                            "method":"PATCH",
                            "path":"/Groups/{group_id}",
                            "data":{{
                                "schemas":["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
                                "Operations":[{{"op":"replace","path":"displayName","value":"Bulk Patched Team"}}]
                            }}
                        }},
                        {{"method":"GET","path":"/Users/{user_id}"}},
                        {{"method":"GET","path":"/Groups/{group_id}"}},
                        {{"method":"DELETE","path":"/Groups/{group_id}"}},
                        {{"method":"DELETE","path":"/Users/{user_id}"}}
                    ]
                }}"#
            ),
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"][0]["status"]["code"], 200);
    assert_eq!(body["Operations"][1]["status"]["code"], 200);
    assert_eq!(
        body["Operations"][2]["response"]["userName"],
        "bulk-put-updated@example.com"
    );
    assert_eq!(body["Operations"][2]["response"]["title"], "Updated");
    assert_eq!(
        body["Operations"][3]["response"]["displayName"],
        "Bulk Patched Team"
    );
    assert_eq!(body["Operations"][4]["status"]["code"], 204);
    assert_eq!(body["Operations"][5]["status"]["code"], 204);
}

#[tokio::test]
async fn bulk_route_executes_user_patch_operations() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);
    let user_id =
        create_scim_user(&router, &token, "bulk-user-patch@example.com", "Bulk Patch").await;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &format!(
                r#"{{
                    "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                    "Operations":[
                        {{
                            "method":"PATCH",
                            "path":"/Users/{user_id}",
                            "data":{{
                                "schemas":["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
                                "Operations":[
                                    {{"op":"replace","path":"title","value":"Principal Engineer"}},
                                    {{"op":"add","path":"phoneNumbers","value":[{{"value":"+15551234567","type":"work"}}]}}
                                ]
                            }}
                        }},
                        {{"method":"GET","path":"/Users/{user_id}"}}
                    ]
                }}"#
            ),
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"][0]["status"]["code"], 200);
    assert!(body["Operations"][0]["version"].as_str().is_some());
    assert_eq!(
        body["Operations"][1]["response"]["title"],
        "Principal Engineer"
    );
    assert_eq!(
        body["Operations"][1]["response"]["phoneNumbers"][0]["value"],
        "+15551234567"
    );
}

#[tokio::test]
async fn bulk_route_rejects_stale_operation_version() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);
    let created = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Users",
            r#"{"userName":"bulk-version@example.com"}"#,
            Some(&token),
        ))
        .await
        .expect("request should succeed");
    let user_id = json_body(created)["id"].as_str().expect("id").to_owned();

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &format!(
                r#"{{
                    "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                    "Operations":[
                        {{
                            "method":"PUT",
                            "path":"/Users/{user_id}",
                            "version":"W/\"stale\"",
                            "data":{{"userName":"bulk-version-updated@example.com"}}
                        }}
                    ]
                }}"#
            ),
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"][0]["status"]["code"], 412);
    assert_eq!(body["Operations"][0]["response"]["status"], "412");
}

#[tokio::test]
async fn bulk_route_rejects_unresolved_bulk_id_references() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            r#"{
                "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
                "Operations":[{"method":"GET","path":"bulkId:missing-user"}]
            }"#,
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response);
    assert_eq!(body["Operations"][0]["status"]["code"], 400);
    assert_eq!(
        body["Operations"][0]["response"]["scimType"],
        "invalidValue"
    );
}

#[tokio::test]
async fn bulk_route_enforces_advertised_operation_limit() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);
    let operations = (0..1001)
        .map(|index| format!(r#"{{"method":"GET","path":"/Users/missing-{index}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        r#"{{
            "schemas":["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
            "Operations":[{operations}]
        }}"#
    );

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &body,
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response);
    assert_eq!(body["scimType"], "tooMany");
}

#[tokio::test]
async fn bulk_route_enforces_advertised_payload_limit() {
    let (adapter, router) = router_with_adapter().expect("router should build");
    ScimProviderStore::new(adapter.as_ref())
        .create(CreateScimProviderInput {
            provider_id: "okta".to_owned(),
            scim_token: "base-token".to_owned(),
            organization_id: None,
            user_id: None,
        })
        .await
        .expect("provider should create");
    let token = encode_bearer_token("base-token", "okta", None);
    let oversized_body = "x".repeat(1_048_577);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/scim/v2/Bulk",
            &oversized_body,
            Some(&token),
        ))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response);
    assert_eq!(body["scimType"], "tooMany");
}
