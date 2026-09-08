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

    const apps = Array.isArray(installed) ? installed.slice() : [];

    // Fallback apps if real apps list is still loading or on emulator
    if (apps.length === 0) {
        const mockData = [
            { name: "Settings", pkg: "com.android.settings" },
            { name: "Camera", pkg: "com.android.camera" },
            { name: "Contacts", pkg: "com.android.contacts" },
            { name: "Browser", pkg: "com.android.chrome" },
            { name: "Calculator", pkg: "com.android.calculator2" },
            { name: "Gallery", pkg: "com.android.gallery3d" },
            { name: "Files", pkg: "com.android.documentsui" },
            { name: "Clock", pkg: "com.android.deskclock" },
            { name: "Messages", pkg: "com.android.mms" },
            { name: "Phone", pkg: "com.android.dialer" }
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

    // 2-Floor Architecture State: "top" = Home Screen, "bottom" = App Drawer
    currentFloor: "top",
    drawerSearchQuery: "",
    showDevKitHud: typeof host_is_devkit_enabled === "function" && host_is_devkit_enabled(),

    allApps: loadInitialApps(),

    // Alphabetical apps list (A-Z) for the Bottom Floor (App Drawer)
    get alphabeticalApps() {
        const list = this.allApps.slice();
        list.sort(function(a, b) {
            const nameA = (a.name || "").toLowerCase();
            const nameB = (b.name || "").toLowerCase();
            return nameA.localeCompare(nameB);
        });
        return list;
    },

    // Filtered apps in App Drawer based on drawerSearchQuery
    get filteredDrawerApps() {
        const q = (this.drawerSearchQuery || "").trim().toLowerCase();
        if (!q) {
            return this.alphabeticalApps;
        }
        return this.alphabeticalApps.filter(function(app) {
            const name = (app.name || "").toLowerCase();
            const pkg = (app.package_name || "").toLowerCase();
            return name.includes(q) || pkg.includes(q);
        });
    },

    goToTopFloor() {
        if (this.currentFloor !== "top") {
            this.currentFloor = "top";
            this.drawerSearchQuery = "";
            this.isSearchFocused = false;
            if (typeof blurInput === "function") {
                blurInput();
            }
            if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
                SnifferUI.forceUpdate();
            }
        }
    },

    goToBottomFloor() {
        if (this.currentFloor !== "bottom") {
            this.currentFloor = "bottom";
            this.rebuildCardHashCache();
            if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
                SnifferUI.forceUpdate();
            }
        }
    },

    toggleFloor() {
        if (this.currentFloor === "bottom") {
            this.goToTopFloor();
        } else {
            this.goToBottomFloor();
        }
    },

    // Search-filtered apps query (microsecond latency via Rust DataVault)
    get filteredApps() {
        const query = (this.searchQuery || "").trim();
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({ search: query, page: 0, limit: 999 });
            if (res && Array.isArray(res.apps) && res.apps.length > 0) {
                return res.apps;
            }
        }

        const q = query.toLowerCase();
        return this.allApps.filter(app => {
            if (!q) return true;
            return (app.name && app.name.toLowerCase().includes(q))
                || (app.package_name && app.package_name.toLowerCase().includes(q));
        });
    },

    // Returns apps for a specific page index (0, 1, 2...)
    appsForPage(pageIndex) {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({
                search: (this.searchQuery || "").trim(),
                page: pageIndex,
                limit: this.appsPerPage
            });
            if (res && Array.isArray(res.apps) && res.apps.length > 0) {
                return res.apps;
            }
        }
        const list = this.filteredApps;
        const startIndex = pageIndex * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    },

    // Returns apps for the currently active page
    get apps() {
        return this.appsForPage(this.currentPage);
    },

    get totalPages() {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({
                search: (this.searchQuery || "").trim(),
                page: 0,
                limit: this.appsPerPage
            });
            if (res && res.total_pages > 0) {
                return res.total_pages;
            }
        }
        return Math.max(1, Math.ceil(this.filteredApps.length / this.appsPerPage));
    },

    // Page navigation helper with bounds check
    setPage(p) {
        if (p >= 0 && p < this.totalPages) {
            this.currentPage = p;
        }
    },

    // Reload applications list
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

        // Index Bottom Floor (App Drawer) cards
        const drawerApps = this.filteredDrawerApps;
        for (let d = 0; d < drawerApps.length; d++) {
            const dPkg = drawerApps[d].package_name;
            if (dPkg) {
                const cId = "drawer_card_" + d;
                const icId = "drawer_icon_" + d;
                const imId = "drawer_img_" + d;
                const nId = "drawer_name_" + d;
                const lId = "drawer_letter_" + d;
                if (typeof host_hash === "function") {
                    map[host_hash(cId)] = dPkg;
                    map[host_hash(icId)] = dPkg;
                    map[host_hash(imId)] = dPkg;
                    map[host_hash(nId)] = dPkg;
                    map[host_hash(lId)] = dPkg;
                } else {
                    map[cId] = dPkg;
                    map[icId] = dPkg;
                    map[imId] = dPkg;
                    map[nId] = dPkg;
                    map[lId] = dPkg;
                }
            }
        }

        this._cardHashToAppPackage = map;
    }
};
state.rebuildCardHashCache();

if (typeof subscribeChannel === "function") {
    subscribeChannel("vault.changed:system.apps", function() {
        state.refreshApps();
        if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
            SnifferUI.forceUpdate();
        }
    });
    subscribeChannel("system.apps", function() {
        state.refreshApps();
        if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
            SnifferUI.forceUpdate();
        }
    });
} else if (typeof Vault !== "undefined" && typeof Vault.subscribe === "function") {
    Vault.subscribe("system.apps", function() {
        state.refreshApps();
        if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
            SnifferUI.forceUpdate();
        }
    });
}
