use sniffer_platform_desktop::get_application_list;

#[test]
fn test_desktop_mock_application_list() {
    let apps = get_application_list().expect("Should return mock apps on desktop");
    assert!(!apps.is_empty());
    assert_eq!(apps.len(), 10);

    let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"Browser"));
    assert!(names.contains(&"Calculator"));
    assert!(names.contains(&"Settings"));
    assert!(names.contains(&"Files"));
}
