#[cfg(not(target_os = "android"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    snifferlauncher::desktop::run()
}

#[cfg(target_os = "android")]
#[allow(clippy::missing_const_for_fn)]
fn main() {}
