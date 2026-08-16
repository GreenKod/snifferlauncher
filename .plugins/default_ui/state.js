// Default UI - State & Page Management (4x7 Layout with Native Data Vault)

function loadInitialApps() {
    let installed = [];
    try {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({ limit: 999 });
            installed = res.apps;
        } else if (typeof getApplicationList === "function") {
            installed = getApplicationList();
        }
    } catch (e) {
        installed = [];
    }

    const apps = Array.isArray(installed) ? [...installed] : [];

    // Fallback apps if real apps list is still loading or on emulator
    if (apps.length === 0) {
        const mockData = [
            { name: "Ayarlar", pkg: "com.android.settings" },
            { name: "Kamera", pkg: "com.android.camera" },
            { name: "Rehber", pkg: "com.android.contacts" },
            { name: "Tarayıcı", pkg: "com.android.chrome" },
            { name: "Hesap Makinesi", pkg: "com.android.calculator2" },
            { name: "Galeri", pkg: "com.android.gallery3d" },
            { name: "Dosyalar", pkg: "com.android.documentsui" },
            { name: "Saat", pkg: "com.android.deskclock" },
            { name: "Mesajlar", pkg: "com.android.mms" },
            { name: "Telefon", pkg: "com.android.dialer" }
        ];

        for (let i = 0; i < mockData.length; i++) {
            apps.push({
                id: "mock_app_" + (i + 1),
                name: mockData[i].name,
                package_name: mockData[i].pkg
            });
        }
    }

    return apps;
}

const APPS_PER_PAGE = 28;

let state = {
    bgColor: null,
    searchQuery: "",
    isSearchFocused: false,
    scrollX: 0.0,
    scrollY: 0.0,

    // Carousel Paging State (0-indexed: 0 = Page 1, 1 = Page 2, ...)
    currentPage: 0,
    appsPerPage: APPS_PER_PAGE,

    allApps: loadInitialApps(),

    // Arama filtreli tüm uygulamalar (Rust DataVault ile mikro-saniye hızında)
    get filteredApps() {
        const query = (this.searchQuery || "").trim();
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            return Vault.queryApps({ search: query, page: 0, limit: 999 }).apps;
        }

        const q = query.toLowerCase();
        return this.allApps.filter(app => {
            if (!q) return true;
            return (app.name && app.name.toLowerCase().includes(q))
                || (app.package_name && app.package_name.toLowerCase().includes(q));
        });
    },

    // Herhangi bir sayfa indeksi (0, 1, 2...) için uygulamaları döndürür
    appsForPage(pageIndex) {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            return Vault.queryApps({
                search: (this.searchQuery || "").trim(),
                page: pageIndex,
                limit: this.appsPerPage
            }).apps;
        }
        const list = this.filteredApps;
        const startIndex = pageIndex * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    },

    // Aktif sayfadaki uygulamaları döndürür
    get apps() {
        return this.appsForPage(this.currentPage);
    },

    get totalPages() {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            return Vault.queryApps({
                search: (this.searchQuery || "").trim(),
                page: 0,
                limit: this.appsPerPage
            }).total_pages;
        }
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
        let pkg = this._cardHashToAppPackage ? this._cardHashToAppPackage[hashStr] : null;
        if (!pkg) {
            this.rebuildCardHashCache();
            pkg = this._cardHashToAppPackage ? this._cardHashToAppPackage[hashStr] : null;
        }
        return pkg;
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
                const nameId = "p" + p + "_name_" + i;
                const letterId = "p" + p + "_letter_" + i;
                const pkg = pageApps[i].package_name;
                if (pkg) {
                    if (typeof host_hash === "function") {
                        map[host_hash(cardId)] = pkg;
                        map[host_hash(iconId)] = pkg;
                        map[host_hash(imgId)] = pkg;
                        map[host_hash(nameId)] = pkg;
                        map[host_hash(letterId)] = pkg;
                    } else {
                        map[cardId] = pkg;
                        map[iconId] = pkg;
                        map[imgId] = pkg;
                        map[nameId] = pkg;
                        map[letterId] = pkg;
                    }
                }
            }
        }
        this._cardHashToAppPackage = map;
    }
};
state.rebuildCardHashCache();
