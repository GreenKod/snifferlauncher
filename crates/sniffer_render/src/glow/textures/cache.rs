#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrimLevel {
    /// Evict until under max_vram_bytes
    Normal,
    /// Evict 50% oldest unpinned textures under memory pressure
    Moderate,
    /// Evict all non-system textures
    Critical,
}

pub struct TextureHandle {
    pub texture: glow::Texture,
    pub width: f32,
    pub height: f32,
    pub size_bytes: usize,
    pub last_frame: u64,
    pub(crate) prev: Option<usize>,
    pub(crate) next: Option<usize>,
}

pub struct LruTextureCache {
    nodes: Vec<Option<TextureHandle>>,
    keys: Vec<Option<String>>,
    free_indices: Vec<usize>,
    map: std::collections::HashMap<String, usize>,
    head: Option<usize>, // Most recently used
    tail: Option<usize>, // Least recently used
    pub total_vram_bytes: usize,
    pub max_vram_bytes: usize,
    pub max_textures: usize,
    pub current_frame: u64,
}

impl Default for LruTextureCache {
    fn default() -> Self {
        Self {
            nodes: Vec::with_capacity(128),
            keys: Vec::with_capacity(128),
            free_indices: Vec::new(),
            map: std::collections::HashMap::with_capacity(128),
            head: None,
            tail: None,
            total_vram_bytes: 0,
            max_vram_bytes: 128 * 1024 * 1024, // 128 MB VRAM limit
            max_textures: 1024,
            current_frame: 0,
        }
    }
}

impl LruTextureCache {
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn detach(&mut self, idx: usize) {
        let (prev, next) = match self.nodes.get(idx).and_then(Option::as_ref) {
            Some(node) => (node.prev, node.next),
            None => return,
        };

        if let Some(p) = prev {
            if let Some(Some(p_node)) = self.nodes.get_mut(p) {
                p_node.next = next;
            }
        } else {
            self.head = next;
        }

        if let Some(n) = next {
            if let Some(Some(n_node)) = self.nodes.get_mut(n) {
                n_node.prev = prev;
            }
        } else {
            self.tail = prev;
        }

        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.prev = None;
            node.next = None;
        }
    }

    fn attach_head(&mut self, idx: usize) {
        let old_head = self.head;
        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.prev = None;
            node.next = old_head;
        }

        if let Some(h) = old_head {
            if let Some(Some(h_node)) = self.nodes.get_mut(h) {
                h_node.prev = Some(idx);
            }
        } else {
            self.tail = Some(idx);
        }

        self.head = Some(idx);
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TextureHandle> {
        let idx = *self.map.get(key)?;
        self.detach(idx);
        self.attach_head(idx);
        if let Some(Some(node)) = self.nodes.get_mut(idx) {
            node.last_frame = self.current_frame;
            Some(node)
        } else {
            None
        }
    }

    pub fn pin(&mut self, key: &str, frame: u64) {
        if let Some(&idx) = self.map.get(key) {
            self.detach(idx);
            self.attach_head(idx);
            if let Some(Some(node)) = self.nodes.get_mut(idx) {
                node.last_frame = frame;
            }
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<TextureHandle> {
        let idx = self.map.remove(key)?;
        self.detach(idx);

        let handle = self.nodes.get_mut(idx)?.take()?;
        self.keys[idx] = None;
        self.free_indices.push(idx);
        Some(handle)
    }

    pub fn insert(
        &mut self,
        key: String,
        texture: glow::Texture,
        width: f32,
        height: f32,
        size_bytes: usize,
    ) -> Option<TextureHandle> {
        if let Some(&idx) = self.map.get(&key) {
            self.detach(idx);
            self.attach_head(idx);
            let old = self.nodes.get_mut(idx)?.take();
            self.nodes[idx] = Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            });
            self.detach(idx);
            self.attach_head(idx);
            return old;
        }

        let idx = if let Some(free_idx) = self.free_indices.pop() {
            self.nodes[free_idx] = Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            });
            self.keys[free_idx] = Some(key.clone());
            free_idx
        } else {
            let new_idx = self.nodes.len();
            self.nodes.push(Some(TextureHandle {
                texture,
                width,
                height,
                size_bytes,
                last_frame: self.current_frame,
                prev: None,
                next: None,
            }));
            self.keys.push(Some(key.clone()));
            new_idx
        };

        self.map.insert(key, idx);
        self.attach_head(idx);
        None
    }

    /// Finds the least-recently used key for eviction, preferring unpinned textures.
    #[must_use]
    pub fn pop_lru_candidate(&self, skip_wallpaper: bool, current_frame: u64) -> Option<String> {
        let mut curr = self.tail;
        let mut fallback: Option<String> = None;

        while let Some(idx) = curr {
            if let (Some(Some(node)), Some(Some(key))) = (self.nodes.get(idx), self.keys.get(idx)) {
                if !skip_wallpaper || key.as_str() != "__system_wallpaper__" {
                    if node.last_frame < current_frame {
                        return Some(key.clone());
                    }
                    if fallback.is_none() {
                        fallback = Some(key.clone());
                    }
                }
                curr = node.prev;
            } else {
                break;
            }
        }

        fallback
    }
}
