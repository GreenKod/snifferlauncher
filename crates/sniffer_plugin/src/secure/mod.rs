/// Previously XOR-obfuscated asset decryption. Removed — single-byte XOR
/// provides no meaningful security and has been replaced with a no-op.
#[must_use]
pub fn decrypt(src: &str) -> &str {
    src
}
