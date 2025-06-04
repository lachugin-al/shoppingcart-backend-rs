use app_config::AppConfig;

#[test]
fn test_load_default_config() {
    let cfg = AppConfig::load().unwrap();
    assert_eq!(cfg.db_host, "postgres");
}
