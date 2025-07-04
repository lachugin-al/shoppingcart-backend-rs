use app_config::AppConfig;

#[test]
fn test_load_default_config() {
    let cfg = AppConfig::load().unwrap();
    // Default is now "postgres" for Docker Compose compatibility
    assert_eq!(cfg.db_host, "postgres");
}
