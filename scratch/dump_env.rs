fn main() {
    for (k, v) in std::env::vars() {
        if k.contains("BINDGEN") || k.contains("CC") || k.contains("TARGET") {
            println!("{}={}", k, v);
        }
    }
}
