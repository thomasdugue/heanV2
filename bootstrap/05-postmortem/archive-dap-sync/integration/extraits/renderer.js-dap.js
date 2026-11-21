// Câblage DAP Sync retiré de renderer.js par le commit 47acae0

// DAP Sync
import { initDapSync, openSyncPanel, closeSyncPanel, loadDestinations as loadDapDestinations, refreshMountedVolumes, displayDapSyncView, hideDapTopBar, renderSidebarDestinations, hasDapDestination, getMountedDestinations, isAlbumSelectedForDap, isArtistFullySelectedForDap, toggleAlbumDapSelection, toggleArtistDapSelection, toggleAlbumsOnDest, showDapSyncModal, getDapDestinationName, startSync as startDapSync } from './dap-sync.js'
// DAP Sync
app.openSyncPanel = openSyncPanel
app.closeSyncPanel = closeSyncPanel
app.loadDapDestinations = loadDapDestinations
app.refreshMountedVolumes = refreshMountedVolumes
app.displayDapSyncView = displayDapSyncView
app.hideDapTopBar = hideDapTopBar
app.renderSidebarDestinations = renderSidebarDestinations
app.hasDapDestination = hasDapDestination
app.getMountedDestinations = getMountedDestinations
app.isAlbumSelectedForDap = isAlbumSelectedForDap
app.isArtistFullySelectedForDap = isArtistFullySelectedForDap
app.toggleAlbumDapSelection = toggleAlbumDapSelection
app.toggleArtistDapSelection = toggleArtistDapSelection
app.toggleAlbumsOnDest = toggleAlbumsOnDest
app.showDapSyncModal = showDapSyncModal
app.getDapDestinationName = getDapDestinationName
app.startDapSync = startDapSync

  // DAP Sync
  initDapSync()

