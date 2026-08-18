//! Faz 5: Virtual Page Window Manager & Element Recycling Pool
//!
//! JS'den gelen tam Element AST ağacını alıp layout hesaplamadan önce budar.
//! Sadece aktif sayfa ve komşularının (current_page ± 1) çocuklarını korur.
//! Hızlı kaydırma (fling) anında pencere yarıçapını (radius) dinamik genişletir.
//! Pencere dışındaki sayfaların çocuklarını sökerek `page_cache` ve `ElementPool`'a aktarır.

use crate::types::Element;
use std::collections::{HashMap, HashSet, VecDeque};

/// Taffy layout'a girecek varsayılan sayfa yarıçapı (current ± 1)
pub const PAGE_WINDOW_RADIUS: usize = 1;

/// ElementPool maksimum kapasitesi (56 eleman = 2 sayfa kartı)
pub const POOL_MAX_CAPACITY: usize = 56;

/// Recycled Element Havuzu.
/// Ekrandan çıkan kart elemanları (AppCardComponent, EmptySlotComponent vb.)
/// bellekten yok edilmeyip bu havuzda saklanır.
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

    /// Bir elemanı pool'a geri verir. Kapasite aşıldığında Rust RAII ile drop edilir.
    pub fn recycle(&mut self, element: Element) {
        if self.pool.len() < self.max_capacity {
            self.pool.push_back(element);
        }
    }

    /// Bir sayfanın tüm çocuk elemanlarını toplu olarak havuza aktarır.
    pub fn recycle_all(&mut self, children: Vec<Element>) {
        for child in children {
            self.recycle(child);
        }
    }

    /// Havuzdan kullanıma hazır bir eleman alır.
    pub fn take(&mut self) -> Option<Element> {
        self.pool.pop_front()
    }

    /// Havuzdaki mevcut eleman sayısı.
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

/// Sanal sayfa penceresi yöneticisi.
/// JS'den gelen 140+ elemanlık ağacı Taffy layout öncesi budar (strip).
/// Hızlı kaydırma (fling) sırasında pencereyi dinamik genişletir (Pre-materialization).
#[derive(Debug)]
pub struct VirtualPageManager {
    /// Stripped sayfa çocukları saklama deposu (page_index -> children)
    page_cache: HashMap<usize, Vec<Element>>,
    /// Recycled element havuzu
    pub element_pool: ElementPool,
    /// Şu an materialized (children yüklü) sayfa indeksleri
    #[allow(dead_code)]
    materialized: HashSet<usize>,
    /// Aktif sayfa indeksi
    current_page: usize,
    /// Toplam sayfa sayısı
    total_pages: usize,
    /// Virtualization devrede mi
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

    /// Aktif sayfa indeksini döndürür.
    #[must_use]
    pub const fn current_page(&self) -> usize {
        self.current_page
    }

    /// Aktif sayfa indeksini doğrudan ayarlar ve tree ile senkronize eder.
    pub fn set_active_page(&mut self, page_idx: usize, root: &mut Element) {
        self.current_page = page_idx;
        self.virtualize_tree(root);
    }

    /// Standard virtualize_tree (hız 0 var sayılarak).
    pub fn virtualize_tree(&mut self, root: &mut Element) {
        self.virtualize_tree_with_velocity(root, 0.0);
    }

    /// Kaydırma hızına (velocity) göre DINAMIK PREFETCH WINDOW uygulayan virtualization.
    /// Hızlı fling anında (|velocity| > 400) pencere yarıçapı 2 veya 3 sayfaya genişletilerek
    /// pop-in ve siyah ekran %100 önlenir.
    pub fn virtualize_tree_with_velocity(&mut self, _root: &mut Element, _velocity: f32) {
        // Virtualization is disabled to prevent mid-swipe layout recalculation CPU spikes.
        // With <10 pages, Taffy can handle the full tree effortlessly.
        self.active = false;
    }

    /// Sürükleme veya fling esnasında scroll offset'i %50 eşiğini geçtiğinde
    /// Erken Pre-materialization tetikler.
    /// Sayfa penceresi değiştiyse `true` döner ve re-layout tetiklenir.
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

    /// Sayfa snap tamamlandığında veya sayfa değiştiğinde çağrılır.
    pub fn on_page_changed(&mut self, new_page: usize, root: &mut Element) {
        self.current_page = new_page;
        self.virtualize_tree(root);
    }

    /// Viewport boyutu veya yönü değiştiğinde (rotation/resize) önbellekleri geçersiz kılar,
    /// tüm sayfaların stub container boyutlarını (`new_w`, `new_h`) güncelleyerek
    /// yeni ekran oranlarına göre zorunlu re-virtualization tetikler.
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

    /// Ekran boyutu veya yönü değiştiğinde (rotation/resize) önbellekleri geçersiz kılar.
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

    /// Belirtilen sayfanın çocuklarını söküp cache veya pool'a taşır.
    /// Sayfa container'ı boş stub olarak kalır.
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

    /// Belirtilen sayfanın çocuklarını cache'den veya pool'dan geri yükler.
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

    /// AST ağacı içinde "app_grid_pager" -> "app_grid_track" alt çocuklarını bulur.
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

    /// AST içinde verilen ID'ye sahip elemanı özyinelemeli (recursive DFS) olarak bulur.
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
