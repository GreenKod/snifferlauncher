// Default UI - State & Page Management (4x7 Layout with Native Data Vault)
// Architecture: Decoupled into FloorManager (Navigation Lifecycle) & AppRepository (Data & Hitbox Indexing)

const APPS_PER_PAGE = 28;

// =============================================================================
// FloorManager - 2-Floor Navigation & Drawer Lifecycle Management
// Manages "top" (Home Screen) and "bottom" (App Drawer) state, transitions,
// search query focus, and unmount timeouts.
// =============================================================================
class FloorManager {
    constructor() {
        this.currentFloor = "top";
        this.drawerSearchQuery = "";
        this.isDrawerMounted = false;
        this.isSearchFocused = false;
        this._drawerCloseTimer = null;
    }

    get isBottomFloor() {
        return this.currentFloor === "bottom";
    }

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
            if (this._drawerCloseTimer) {
                clearTimeout(this._drawerCloseTimer);
            }
            const self = this;
            this._drawerCloseTimer = setTimeout(function() {
                if (self.currentFloor === "top") {
                    self.isDrawerMounted = false;
                    if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
                        SnifferUI.forceUpdate();
                    }
                }
                self._drawerCloseTimer = null;
            }, 230);
        }
    }

    goToBottomFloor(onOpen) {
        if (this._drawerCloseTimer) {
            clearTimeout(this._drawerCloseTimer);
            this._drawerCloseTimer = null;
        }
        this.isDrawerMounted = true;
        if (this.currentFloor !== "bottom") {
            this.currentFloor = "bottom";
            if (typeof onOpen === "function") {
                onOpen();
            }
            if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
                SnifferUI.forceUpdate();
            }
        }
    }

    toggleFloor(onOpen) {
        if (this.currentFloor === "bottom") {
            this.goToTopFloor();
        } else {
            this.goToBottomFloor(onOpen);
        }
    }

    onBack() {
        if (this.currentFloor === "bottom") {
            this.goToTopFloor();
            return true;
        }
        return false;
    }
}

// =============================================================================
// AppRepository - Application Data Source, Caching & Hitbox Indexing
// Integrates with native DataVault, manages mock fallback data, alphabetical
// sorting, search queries, pagination, and card hitbox hash mappings.
// =============================================================================
class AppRepository {
    constructor(appsPerPage = APPS_PER_PAGE) {
        this.appsPerPage = appsPerPage;
        this.allApps = this.loadInitialApps();
        this._cachedAlphabeticalApps = null;
        this._cardHashToAppPackage = null;
    }

    loadInitialApps() {
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

    get alphabeticalApps() {
        if (!this._cachedAlphabeticalApps) {
            const list = this.allApps.slice();
            list.sort(function(a, b) {
                const nameA = a.name || "";
                const nameB = b.name || "";
                return nameA.localeCompare(nameB, "tr", { sensitivity: "base" });
            });
            this._cachedAlphabeticalApps = list;
        }
        return this._cachedAlphabeticalApps;
    }

    getFilteredDrawerApps(query = "") {
        const q = (query || "").trim().toLowerCase();
        if (!q) {
            return this.alphabeticalApps;
        }
        return this.alphabeticalApps.filter(function(app) {
            const name = (app.name || "").toLowerCase();
            const pkg = (app.package_name || "").toLowerCase();
            return name.includes(q) || pkg.includes(q);
        });
    }

    getFilteredApps(searchQuery = "") {
        const query = (searchQuery || "").trim();
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
    }

    appsForPage(pageIndex, searchQuery = "") {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({
                search: (searchQuery || "").trim(),
                page: pageIndex,
                limit: this.appsPerPage
            });
            if (res && Array.isArray(res.apps) && res.apps.length > 0) {
                return res.apps;
            }
        }
        const list = this.getFilteredApps(searchQuery);
        const startIndex = pageIndex * this.appsPerPage;
        return list.slice(startIndex, startIndex + this.appsPerPage);
    }

    getTotalPages(searchQuery = "") {
        if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
            const res = Vault.queryApps({
                search: (searchQuery || "").trim(),
                page: 0,
                limit: this.appsPerPage
            });
            if (res && res.total_pages > 0) {
                return res.total_pages;
            }
        }
        return Math.max(1, Math.ceil(this.getFilteredApps(searchQuery).length / this.appsPerPage));
    }

    refreshApps() {
        if (typeof _dragCacheGrids !== 'undefined') _dragCacheGrids = null;
        this._cardHashToAppPackage = null;
        this._cachedAlphabeticalApps = null;
        this.allApps = this.loadInitialApps();
    }

    getAppPackageByHash(hashStr, drawerApps) {
        if (!this._cardHashToAppPackage) {
            this.rebuildCardHashCache(drawerApps);
        }
        let pkg = this._cardHashToAppPackage ? this._cardHashToAppPackage[hashStr] : null;
        if (!pkg) {
            this.rebuildCardHashCache(drawerApps);
            pkg = this._cardHashToAppPackage ? this._cardHashToAppPackage[hashStr] : null;
        }
        return pkg;
    }

    rebuildCardHashCache(drawerApps = []) {
        const map = {};
        const total = this.getTotalPages();
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
        const dApps = drawerApps || [];
        for (let d = 0; d < dApps.length; d++) {
            const dPkg = dApps[d].package_name;
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
}

// =============================================================================
// State Coordinator (Unified Facade)
// Maintains 100% backward compatibility for all components & scripts.
// =============================================================================
const floorManager = new FloorManager();
const appRepository = new AppRepository(APPS_PER_PAGE);

let state = {
    // FloorManager Delegation
    floorManager,
    get currentFloor() { return this.floorManager.currentFloor; },
    set currentFloor(val) { this.floorManager.currentFloor = val; },
    get drawerSearchQuery() { return this.floorManager.drawerSearchQuery; },
    set drawerSearchQuery(val) { this.floorManager.drawerSearchQuery = val; },
    get isDrawerMounted() { return this.floorManager.isDrawerMounted; },
    set isDrawerMounted(val) { this.floorManager.isDrawerMounted = val; },
    get isSearchFocused() { return this.floorManager.isSearchFocused; },
    set isSearchFocused(val) { this.floorManager.isSearchFocused = val; },
    get _drawerCloseTimer() { return this.floorManager._drawerCloseTimer; },
    set _drawerCloseTimer(val) { this.floorManager._drawerCloseTimer = val; },
    goToTopFloor() { return this.floorManager.goToTopFloor(); },
    goToBottomFloor() { return this.floorManager.goToBottomFloor(() => this.rebuildCardHashCache()); },
    toggleFloor() { return this.floorManager.toggleFloor(() => this.rebuildCardHashCache()); },
    onBack() { return this.floorManager.onBack(); },

    // AppRepository Delegation
    appRepository,
    get allApps() { return this.appRepository.allApps; },
    set allApps(val) { this.appRepository.allApps = val; },
    get alphabeticalApps() { return this.appRepository.alphabeticalApps; },
    get filteredDrawerApps() { return this.appRepository.getFilteredDrawerApps(this.drawerSearchQuery); },
    get filteredApps() { return this.appRepository.getFilteredApps(this.searchQuery); },
    appsForPage(pageIndex) { return this.appRepository.appsForPage(pageIndex, this.searchQuery); },
    get apps() { return this.appsForPage(this.currentPage); },
    get totalPages() { return this.appRepository.getTotalPages(this.searchQuery); },
    refreshApps() { this.appRepository.refreshApps(); },
    getAppPackageByHash(hashStr) { return this.appRepository.getAppPackageByHash(hashStr, this.filteredDrawerApps); },
    rebuildCardHashCache() { this.appRepository.rebuildCardHashCache(this.filteredDrawerApps); },

    // Paging & UI General State
    bgColor: null,
    searchQuery: "",
    scrollX: 0.0,
    scrollY: 0.0,
    currentPage: 0,
    get appsPerPage() { return this.appRepository.appsPerPage; },
    set appsPerPage(val) { this.appRepository.appsPerPage = val; },
    showDevKitHud: typeof host_is_devkit_enabled === "function" && host_is_devkit_enabled(),

    setPage(p) {
        if (p >= 0 && p < this.totalPages) {
            this.currentPage = p;
        }
    },
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
