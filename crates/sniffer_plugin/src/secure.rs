/// Compile-time encrypted assets runtime decryption.
const XOR_KEY: u8 = 0x5A;

#[must_use]
pub fn decrypt(data: &[u8]) -> String {
    let decrypted: Vec<u8> = data.iter().map(|b| b ^ XOR_KEY).collect();
    String::from_utf8(decrypted).unwrap_or_default()
}

