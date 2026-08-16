#![allow(clippy::missing_errors_doc)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "android"))]
    {
        snifferlauncher::platform::desktop::run()?;
    }
    #[cfg(target_os = "android")]
    {
        println!("This binary is for desktop preview only. Use cargo-apk to build for Android.");
    }
    Ok(())
}
