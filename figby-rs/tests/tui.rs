use std::collections::BTreeMap;

const ICONS_YAML: &str = include_str!("../../assets/tui/icons.yaml");

#[test]
fn test_icons_yaml_all_keys_present() {
    let map: BTreeMap<String, String> =
        serde_yaml::from_str(ICONS_YAML).expect("failed to parse icons.yaml");

    assert!(
        map.len() >= 120,
        "expected at least 120 icon entries, got {}",
        map.len()
    );

    for (key, value) in &map {
        assert!(!key.is_empty(), "empty key found");
        assert!(
            value.starts_with("nf-"),
            "icon value for '{}' does not start with 'nf-': got '{}'",
            key,
            value
        );
    }
}
