/// Runtime XOR decryption for assets encrypted by `build.rs`.
///
/// The XOR key **must** match the `XOR_KEY` constant in `build.rs`.
/// Decrypted data lives only on the stack / heap for the duration of its use;
/// it is never stored as a static string in the binary.
const XOR_KEY: u8 = 0x5A;

/// Decrypt a compile-time XOR-encrypted byte slice into a `String`.
///
/// # Panics
/// Never panics — invalid UTF-8 bytes are replaced with the Unicode replacement
/// character (`U+FFFD`) so GLSL / JS parsing can still fail gracefully.
#[inline]
pub fn decrypt(data: &[u8]) -> String {
    let decrypted: Vec<u8> = data.iter().map(|b| b ^ XOR_KEY).collect();
    String::from_utf8_lossy(&decrypted).into_owned()
}
