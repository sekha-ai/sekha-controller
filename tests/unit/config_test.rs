use sekha_controller::config::Config;
use validator::Validate;

#[test]
fn test_config_validation() {
    let config = Config::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_has_required_fields() {
    let config = Config::default();
    assert!(!config.mcp_api_key.is_empty());
    assert!(!config.database_url.is_empty());
    assert!(!config.chroma_url.is_empty());
}
