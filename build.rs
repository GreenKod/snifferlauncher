use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=.plugins");

    // Copy .plugins directory to build profile output directory (target/debug/.plugins or target/release/.plugins)
    let src_plugins = Path::new(".plugins");
    if src_plugins.exists() {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let out_dir_path = Path::new(&out_dir);
            if let Some(profile_dir) = out_dir_path.ancestors().nth(3) {
                let target_plugins = profile_dir.join(".plugins");
                let _ = copy_dir_all(src_plugins, &target_plugins);
            }
        }
        let debug_plugins = Path::new("target/debug/.plugins");
        let release_plugins = Path::new("target/release/.plugins");
        if Path::new("target/debug").exists() {
            let _ = copy_dir_all(src_plugins, debug_plugins);
        }
        if Path::new("target/release").exists() {
            let _ = copy_dir_all(src_plugins, release_plugins);
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    let dst_path = dst.as_ref();
    fs::create_dir_all(dst_path)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_entry = dst_path.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(entry.path(), &dest_entry)?;
        } else {
            let _ = fs::copy(entry.path(), &dest_entry);
        }
    }
    Ok(())
}
