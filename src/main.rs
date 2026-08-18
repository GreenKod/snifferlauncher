fn main() {
    #[cfg(not(target_os = "android"))]
    {
        if let Err(e) = sniffer_platform_desktop::run() {
            eprintln!("Error running Sniffer desktop: {e}");
            std::process::exit(1);
        }
    }
    #[cfg(target_os = "android")]
    {
        println!(
            "This binary is for desktop preview only. Use cargo-apk or cargo-ndk to build for Android."
        );
    }
}
