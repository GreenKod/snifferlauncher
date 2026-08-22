use sniffer_render::glow::textures::LruTextureCache;
use std::num::NonZeroU32;

fn dummy_texture(id: u32) -> glow::Texture {
    glow::NativeTexture(NonZeroU32::new(id).unwrap())
}

#[test]
fn test_lru_cache_basic_insert_and_contains() {
    let mut cache = LruTextureCache::default();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);

    cache.insert("icon_1".to_string(), dummy_texture(1), 96.0, 96.0, 96 * 96 * 4);
    assert!(!cache.is_empty());
    assert_eq!(cache.len(), 1);
    assert!(cache.contains_key("icon_1"));
    assert!(!cache.contains_key("icon_2"));
}

#[test]
fn test_lru_cache_eviction_order() {
    let mut cache = LruTextureCache::default();
    cache.max_textures = 3;

    // Insert 3 textures at frame 1
    cache.current_frame = 1;
    cache.insert("icon_a".to_string(), dummy_texture(1), 96.0, 96.0, 100);
    cache.insert("icon_b".to_string(), dummy_texture(2), 96.0, 96.0, 100);
    cache.insert("icon_c".to_string(), dummy_texture(3), 96.0, 96.0, 100);

    // Touch icon_a at frame 2 so it becomes most recently used
    cache.current_frame = 2;
    let _ = cache.get_mut("icon_a");

    // The least recently used should be icon_b (since icon_a was touched, order is c -> b)
    let candidate = cache.pop_lru_candidate(true, 3);
    assert_eq!(candidate, Some("icon_b".to_string()));
}

#[test]
fn test_lru_cache_wallpaper_protection() {
    let mut cache = LruTextureCache::default();
    cache.insert(
        "__system_wallpaper__".to_string(),
        dummy_texture(99),
        1080.0,
        1920.0,
        1080 * 1920 * 4,
    );
    cache.insert("icon_1".to_string(), dummy_texture(1), 96.0, 96.0, 100);

    // Wallpaper should be skipped when skip_wallpaper is true
    let candidate = cache.pop_lru_candidate(true, 10);
    assert_eq!(candidate, Some("icon_1".to_string()));
}

#[test]
fn test_lru_cache_pinning() {
    let mut cache = LruTextureCache::default();
    cache.current_frame = 10;
    cache.insert("icon_1".to_string(), dummy_texture(1), 96.0, 96.0, 100);
    cache.insert("icon_2".to_string(), dummy_texture(2), 96.0, 96.0, 100);

    // Pin icon_1 to current frame (10)
    cache.pin("icon_1", 10);

    // Eviction seeking unpinned nodes for frame 10 should pick icon_2
    let candidate = cache.pop_lru_candidate(true, 10);
    assert_eq!(candidate, Some("icon_2".to_string()));
}
