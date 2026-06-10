use openauth::plugin::AuthPlugin;
use openauth::OpenAuth;

#[tokio::test]
async fn builder_plugins_extends_existing_plugin_list() -> Result<(), Box<dyn std::error::Error>> {
    let auth = OpenAuth::builder()
        .secret("secret-a-at-least-32-chars-long!!")
        .plugin(AuthPlugin::new("first"))
        .plugins(vec![AuthPlugin::new("second"), AuthPlugin::new("third")])
        .build()
        .await?;

    let ids: Vec<_> = auth
        .options()
        .plugins
        .iter()
        .map(|plugin| plugin.id.as_str())
        .collect();
    assert_eq!(ids, ["first", "second", "third"]);
    Ok(())
}
