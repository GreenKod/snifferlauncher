/// Shader source passthrough — previously XOR-obfuscated, now plain text.
/// Kept as a no-op shim so existing call sites in `glow/mod.rs` compile unchanged.
#[must_use]
pub fn decrypt(src: &str) -> &str {
    src
}
