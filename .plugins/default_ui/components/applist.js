
function getApplist() {
    const apps = host_get_installed_apps();
    const appList = JSON.parse(apps);
    return appList;
}