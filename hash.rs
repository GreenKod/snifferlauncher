fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn main() {
    println!("fab_button: {}", fnv1a(b"fab_button"));
    println!("fab_text: {}", fnv1a(b"fab_text"));
    println!("root: {}", fnv1a(b"root"));
    println!("search_input: {}", fnv1a(b"search_input"));
}
