// Code DAP Sync retiré de views.js par le commit 47acae0

    dom.albumsGridDiv.classList.remove('dap-sync-view') // Remove DAP-specific overflow:hidden on .albums-view
    // Restore search bar and deselect DAP sidebar item when leaving DAP sync view
    if (ui.currentView !== 'dap-sync') {
      app.hideDapTopBar?.()
      app.renderSidebarDestinations?.()
    }
      case 'dap-sync':
        app.displayDapSyncView()
        break
  if (!['home', 'albums', 'artists', 'tracks', 'dap-sync'].includes(view)) return
