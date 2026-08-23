use super::*;

#[test]
fn prepare_android_dylib_creates_dir_and_file() {
    let tmp = std::env::temp_dir().join("sniffer_pkg_loader_test");
    let src_dir = tmp.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    let src_file = src_dir.join("libfoo.so");
    std::fs::write(&src_file, b"ELF").unwrap();

    let internal = tmp.join("internal");
    let dest = prepare_android_dylib(&src_file, &internal).unwrap();

    assert!(dest.exists(), "destination file must exist");
    assert_eq!(dest.file_name().unwrap(), "libfoo.so");

    let _ = std::fs::remove_dir_all(&tmp);
}
