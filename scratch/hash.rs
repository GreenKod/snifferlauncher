fn fnv1a(s: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325_u64;
    for &b in s {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3_u64);
    }
    hash
}
fn main() {
    println!("root: {}", fnv1a(b"root"));
    println!("btn-merhaba: {}", fnv1a(b"btn-merhaba"));
}
