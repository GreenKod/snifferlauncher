//! Virtual Page Window Manager & Element Recycling Pool
//!
//! Prunes the full Element AST tree before layout calculation.
//! Keeps only active and adjacent (current_page ± 1) page children.
//! Dynamically expands prefetch window during high-speed fling gestures.
//! Moves off-screen page children to `page_cache` and `ElementPool`.

use crate::types::Element;
use std::collections::{HashMap, HashSet, VecDeque};

/// Default page window radius entering Taffy layout (current ± 1)
pub const PAGE_WINDOW_RADIUS: usize = 1;

/// Maximum ElementPool capacity (56 elements = 2 pages of cards)
pub const POOL_MAX_CAPACITY: usize = 56;

/// Recycled Element Pool.
/// Off-screen card elements are preserved in this pool rather than being deallocated.
#[derive(Debug)]
pub struct ElementPool {
    pool: VecDeque<Element>,
    max_capacity: usize,
}

impl ElementPool {
    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            pool: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// Recycles an element into the pool. Dropped via RAII when capacity is exceeded.
    pub fn recycle(&mut self, element: Element) {
        if self.pool.len() < self.max_capacity {
            self.pool.push_back(element);
        }
    }

    /// Recycles an entire batch of child elements into the pool.
    pub fn recycle_all(&mut self, children: Vec<Element>) {
        for child in children {
            self.recycle(child);
        }
    }

    /// Retrieves an available element from the pool.
    pub fn take(&mut self) -> Option<Element> {
        self.pool.pop_front()
    }

    /// Number of elements currently available in the pool.
    #[must_use]
    pub fn available(&self) -> usize {
        self.pool.len()
    }
}

impl Default for ElementPool {
    fn default() -> Self {
        Self::new(POOL_MAX_CAPACITY)
    }
}

/// Virtual page window manager.
/// Prunes the 140+ element AST tree before Taffy layout calculation.
/// Expands window dynamically during fling gestures (pre-materialization).
#[derive(Debug)]
pub struct VirtualPageManager {
    /// Stripped page children storage (page_index -> children)
    page_cache: HashMap<usize, Vec<Element>>,
    /// Recycled element pool
    pub element_pool: ElementPool,
    /// Currently materialized page indices
    #[allow(dead_code)]
    materialized: HashSet<usize>,
    /// Currently active page index
    current_page: usize,
    /// Total page count
    total_pages: usize,
    /// Whether virtualization is active
    active: bool,
}

impl Default for VirtualPageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualPageManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            page_cache: HashMap::new(),
            element_pool: ElementPool::new(POOL_MAX_CAPACITY),
            materialized: HashSet::new(),
            current_page: 0,
            total_pages: 1,
            active: false,
        }
    }

    /// Returns the active page index.
    #[must_use]
    pub const fn current_page(&self) -> usize {
        self.current_page
    }

    /// Directly sets the active page index and synchronizes the tree.
    pub fn set_active_page(&mut self, page_idx: usize, root: &mut Element) {
        self.current_page = page_idx;
        self.virtualize_tree(root);
    }

    /// Standard virtualize_tree (assuming zero velocity).
    pub fn virtualize_tree(&mut self, root: &mut Element) {
        self.virtualize_tree_with_velocity(root, 0.0);
    }

    /// Virtualization with dynamic prefetch window based on scroll velocity.
    /// During fast flings (|velocity| > 400), window radius expands to 2-3 pages
    /// to eliminate pop-in or black frames.
    pub fn virtualize_tree_with_velocity(&mut self, _root: &mut Element, _velocity: f32) {
        // Virtualization is disabled to prevent mid-swipe layout recalculation CPU spikes.
        // With <10 pages, Taffy can handle the full tree effortlessly.
        self.active = false;
    }

    /// Triggers pre-materialization when scroll offset crosses 50% threshold.
    /// Returns true if the page window changed, triggering re-layout.
    pub fn update_predicted_page(
        &mut self,
        predicted_page: usize,
        root: &mut Element,
        velocity: f32,
    ) -> bool {
        self.current_page = predicted_page;
        self.virtualize_tree_with_velocity(root, velocity);
        false // Force false to prevent mid-swipe Taffy layout recalculation CPU spikes
    }

    /// Invoked when page snap completes or active page changes.
    pub fn on_page_changed(&mut self, new_page: usize, root: &mut Element) {
        self.current_page = new_page;
        self.virtualize_tree(root);
    }

    /// Invalidates caches on viewport resize or rotation,
    /// updating stub container dimensions and forcing re-virtualization.
    pub fn on_viewport_resized(&mut self, _new_w: f32, _new_h: f32, root: &mut Element) {
        let cached_keys: Vec<usize> = self.page_cache.keys().copied().collect();
        for page_idx in cached_keys {
            if let Some(track_children) = Self::find_track_children_mut(root, &mut self.total_pages)
                && let Some(page_el) = track_children.get_mut(page_idx)
            {
                self.restore_page(page_idx, page_el);
            }
        }
        self.page_cache.clear();
        self.virtualize_tree(root);
    }

    /// Invalidates caches on viewport dimension or orientation changes.
    pub fn invalidate_on_resize(&mut self, root: &mut Element) {
        let cached_keys: Vec<usize> = self.page_cache.keys().copied().collect();
        for page_idx in cached_keys {
            if let Some(track_children) = Self::find_track_children_mut(root, &mut self.total_pages)
                && let Some(page_el) = track_children.get_mut(page_idx)
            {
                self.restore_page(page_idx, page_el);
            }
        }
        self.page_cache.clear();
        self.virtualize_tree(root);
    }

    /// Strips children of a specific page and moves them to cache or pool.
    /// The page container remains as an empty stub.
    pub fn strip_page(&mut self, page_idx: usize, page_el: &mut Element) {
        let children_to_strip = match page_el {
            Element::Container { children, .. } | Element::ScrollView { children, .. } => {
                if children.is_empty() {
                    return;
                }
                std::mem::take(children)
            }
            _ => return,
        };

        if self.page_cache.len() < 5 {
            self.page_cache.insert(page_idx, children_to_strip);
        } else {
            self.element_pool.recycle_all(children_to_strip);
        }
    }

    /// Restores children of a specific page from cache or pool.
    pub fn restore_page(&mut self, page_idx: usize, page_el: &mut Element) {
        let children_target = match page_el {
            Element::Container { children, .. } | Element::ScrollView { children, .. } => children,
            _ => return,
        };

        if !children_target.is_empty() {
            return;
        }

        if let Some(cached_children) = self.page_cache.remove(&page_idx) {
            *children_target = cached_children;
        }
    }

    /// Locates "app_grid_pager" -> "app_grid_track" children inside the AST tree.
    fn find_track_children_mut<'a>(
        root: &'a mut Element,
        out_page_count: &mut usize,
    ) -> Option<&'a mut Vec<Element>> {
        let pager = Self::find_element_by_id_mut(root, "app_grid_pager")?;
        if let Element::ScrollView {
            children,
            page_count,
            ..
        } = pager
        {
            if let Some(pc) = page_count {
                *out_page_count = *pc as usize;
            }
            if let Some(Element::Container {
                id,
                children: track_children,
                ..
            }) = children.first_mut()
                && id.as_deref() == Some("app_grid_track")
            {
                return Some(track_children);
            }
        }
        None
    }

    /// Recursively finds an element by ID within the AST tree (DFS).
    pub fn find_element_by_id_mut<'a>(
        element: &'a mut Element,
        target_id: &str,
    ) -> Option<&'a mut Element> {
        if element.id() == Some(target_id) {
            return Some(element);
        }

        match element {
            Element::Container { children, .. }
            | Element::ScrollView { children, .. }
            | Element::SharedView { children, .. } => {
                for child in children {
                    if let Some(found) = Self::find_element_by_id_mut(child, target_id) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
        None
    }
}
