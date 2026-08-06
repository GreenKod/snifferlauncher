// Default UI - State & Page Management (4x7 Layout)

function loadInitialApps() {
    let installed = [];
    try {
        installed = typeof getApplicationList === "function" ? getApplicationList() : [];
    } catch (e) {
        installed = [];
    }

    const apps = Array.isArray(installed) ? [...installed] : [];

    // Only generate mock apps if NO real apps were returned (e.g. desktop/emulator fallback)
    if (apps.length === 0) {
        const mockNames = [
            "Borsa & Finans", "Ses Kaydedici", "Kamera Pro", "Video Oynatıcı", "Oyun Parkı", "E-Posta Client", "Radyo FM", "Podkast Player",
            "Rehber Sync", "Sistem Monitörü", "Güvenlik Duvarı", "Sosyal Medya", "Sohbet Odası", "Haberler 24", "Kitap Okuyucu",
            "Fitness Tracker", "Sağlık Koçu", "Dijital Cüzdan", "Foto Düzenleyici", "Ses Ayarları", "Şifre Kasası", "Harita Gezgini", "Bulut Sürücü",
            "Çizim Tahtası", "Kod Editörü", "Cümle Çeviri", "Görev Listesi", "Anımsatıcılar", "Hava Kirliliği", "Kronometre Pro", "Pusula HD"
        ];

        for (let i = 0; i < mockNames.length; i++) {
            apps.push({
                id: "mock_app_" + (i + 1),
                name: mockNames[i],
                package_name: null
            });
        }
    }

    return apps;
}

const APPS_PER_PAGE = 28;

let state = {
    bgColor: "#0F0F12",
    searchQuery: "",
    isSearchFocused: false,
    scrollX: 0.0,
    scrollY: 0.0,

    // Carousel Paging State (0-indexed: 0 = Page 1, 1 = Page 2, ...)
    currentPage: 0,
    appsPerPage: APPS_PER_PAGE,
    // Not: isDragging ve dragOffset kaldırıldı — Rust fizik motoru yönetiyor

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

    // Herhangi bir sayfa indeksi (0, 1, 2...) için uygulamaları döndürür
    appsForPage(pageIndex) {
        const list = this.filteredApps;
        const startIndex = pageIndex * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    },

    // Aktif sayfadaki uygulamaları döndürür
    get apps() {
        return this.appsForPage(this.currentPage);
    },

    get totalPages() {
        return Math.max(1, Math.ceil(this.filteredApps.length / this.appsPerPage));
    },

    // Sayfa değiştirme fonksiyonu (0-indexed sınır kontrolü)
    setPage(p) {
        if (p >= 0 && p < this.totalPages) {
            this.currentPage = p;
        }
    },

    // Uygulamalar listesini yeniden yükleme
    refreshApps() {
        if (typeof _dragCacheGrids !== 'undefined') _dragCacheGrids = null;
        this._cardHashToAppPackage = null;
        this.allApps = loadInitialApps();
    },

    getAppPackageByHash(hashStr) {
        if (!this._cardHashToAppPackage) {
            this.rebuildCardHashCache();
        }
        return this._cardHashToAppPackage ? this._cardHashToAppPackage[hashStr] : null;
    },

    rebuildCardHashCache() {
        const map = {};
        const total = this.totalPages;
        for (let p = 0; p < total; p++) {
            const pageApps = this.appsForPage(p);
            for (let i = 0; i < pageApps.length; i++) {
                const cardId = "p" + p + "_card_" + i;
                const iconId = "p" + p + "_icon_" + i;
                const imgId = "p" + p + "_img_" + i;
                const pkg = pageApps[i].package_name;
                if (pkg) {
                    if (typeof host_hash === "function") {
                        map[host_hash(cardId)] = pkg;
                        map[host_hash(iconId)] = pkg;
                        map[host_hash(imgId)] = pkg;
                    } else {
                        map[cardId] = pkg;
                        map[iconId] = pkg;
                        map[imgId] = pkg;
                    }
                }
            }
        }
        this._cardHashToAppPackage = map;
    }
};
