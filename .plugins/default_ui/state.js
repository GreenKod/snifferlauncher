// Default UI - State & Page Management (4x7 Layout)

function loadInitialApps() {
    let installed = [];
    try {
        installed = typeof getApplicationList === "function" ? getApplicationList() : [];
    } catch (e) {
        installed = [];
    }

    if (Array.isArray(installed) && installed.length > 0) {
        return installed;
    }

    return [
        { id: "app_1", name: "Tarayıcı", package_name: "com.sniffer.browser" },
        { id: "app_2", name: "Ayarlar", package_name: "com.sniffer.settings" },
        { id: "app_3", name: "Galeri", package_name: "com.sniffer.gallery" },
        { id: "app_4", name: "Kamera", package_name: "com.sniffer.camera" },
        { id: "app_5", name: "Müzik", package_name: "com.sniffer.music" },
        { id: "app_6", name: "Dosyalar", package_name: "com.sniffer.files" },
        { id: "app_7", name: "Terminal", package_name: "com.sniffer.terminal" },
        { id: "app_8", name: "Hesap Makinesi", package_name: "com.sniffer.calc" },
        { id: "app_9", name: "Takvim", package_name: "com.sniffer.calendar" },
        { id: "app_10", name: "Saat", package_name: "com.sniffer.clock" },
        { id: "app_11", name: "Mesajlar", package_name: "com.sniffer.messages" },
        { id: "app_12", name: "Telefon", package_name: "com.sniffer.phone" },
        { id: "app_13", name: "Notlar", package_name: "com.sniffer.notes" },
        { id: "app_14", name: "Haritalar", package_name: "com.sniffer.maps" },
        { id: "app_15", name: "Mağaza", package_name: "com.sniffer.store" },
        { id: "app_16", name: "Hava Durumu", package_name: "com.sniffer.weather" }
    ];
}

const APPS_PER_PAGE = 28;

let state = {
    bgColor: "#0F0F12",
    searchQuery: "",
    isSearchFocused: false,
    scrollX: 0.0,
    scrollY: 0.0,

    // Pagination Mantığı: Sayfa 1 varsayılan; currentPage = 2 yapılınca sonraki 28 uygulama gelir
    currentPage: 1,
    appsPerPage: APPS_PER_PAGE,

    allApps: loadInitialApps(),

    // Arama filtreli tüm uygulamalar
    get filteredApps() {
        const query = (this.searchQuery || "").toLowerCase().trim();
        return this.allApps.filter(app => {
            if (!query) return true;
            return (app.name && app.name.toLowerCase().includes(query))
                || (app.package_name && app.package_name.toLowerCase().includes(query));
        });
    },

    // Seçili sayfadaki uygulamaları döndürür (Örn: currentPage=1 iken 0..27, currentPage=2 iken 28..55)
    get apps() {
        const list = this.filteredApps;
        const startIndex = (this.currentPage - 1) * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    },

    get totalPages() {
        return Math.max(1, Math.ceil(this.filteredApps.length / this.appsPerPage));
    },

    // Sayfa değiştirme fonksiyonu (sayfa sınırlarını korur)
    setPage(p) {
        const maxPage = this.totalPages;
        if (p >= 1 && p <= maxPage) {
            this.currentPage = p;
        }
    }
};
