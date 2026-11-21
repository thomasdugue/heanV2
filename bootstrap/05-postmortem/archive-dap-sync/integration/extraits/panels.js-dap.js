// Code DAP Sync retiré de panels.js par le commit 47acae0

    ${(() => {
      const hasDap = app.hasDapDestination && app.hasDapDestination()
      const isOnDap = hasDap && app.isAlbumSelectedForDap ? app.isAlbumSelectedForDap(albumKey) : false
      const dapLabel = isOnDap ? 'Remove from my DAP' : 'Add to my DAP'
      return `
        <div class="context-menu-separator"></div>
        <button class="context-menu-item ${!hasDap ? 'disabled' : ''}" data-action="toggle-dap-album">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="5" y="1" width="14" height="22" rx="2.5"/>
            <rect x="7" y="3" width="10" height="8" rx="1"/>
            <circle cx="12" cy="17" r="3.5"/>
            <circle cx="12" cy="17" r="1" fill="currentColor" stroke="none"/>
          </svg>
          <span>${dapLabel}</span>
        </button>`
    })()}
  // DAP toggle handler
  const dapBtn = menu.querySelector('[data-action="toggle-dap-album"]')
  if (dapBtn) {
    dapBtn.addEventListener('click', () => {
      if (app.hasDapDestination && app.hasDapDestination()) {
        app.showDapSyncModal({
          albumKeys: [albumKey],
        })
      } else {
        showToast('No DAP device connected')
      }
      menu.remove()
    })
  }
  const hasDap = app.hasDapDestination && app.hasDapDestination()
  const allOnDap = hasDap && app.isArtistFullySelectedForDap ? app.isArtistFullySelectedForDap(artistKey) : false
  const dapLabel = allOnDap ? 'Remove from my DAP' : 'Add to my DAP'

    <div class="context-menu-separator"></div>
    <button class="context-menu-item ${!hasDap ? 'disabled' : ''}" data-action="toggle-dap-artist">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <rect x="5" y="1" width="14" height="22" rx="2.5"/>
        <rect x="7" y="3" width="10" height="8" rx="1"/>
        <circle cx="12" cy="17" r="3.5"/>
        <circle cx="12" cy="17" r="1" fill="currentColor" stroke="none"/>
      </svg>
      <span>${dapLabel}</span>
    </button>
  // DAP toggle
  const dapBtn = menu.querySelector('[data-action="toggle-dap-artist"]')
  if (dapBtn) {
    dapBtn.addEventListener('click', () => {
      if (hasDap) {
        app.showDapSyncModal({
          albumKeys: artist.albums || [],
          artistName: artist.name,
        })
      } else {
        showToast('No DAP device connected')
      }
      menu.remove()
    })
  }


  // DAP toggle button — update label and visibility
  const dapBtn = menu.querySelector('[data-action="toggle-dap-track"]')
  if (dapBtn) {
    const hasDap = app.hasDapDestination && app.hasDapDestination()
    if (!hasDap) {
      dapBtn.classList.add('disabled')
      dapBtn.querySelector('span').textContent = 'Add to my DAP'
    } else {
      dapBtn.classList.remove('disabled')
      // For tracks, check if the track's album is selected for DAP
      const trackAlbum = contextMenu.tracks[0]?.metadata?.album
      if (trackAlbum) {
        // Find the albumKey for this track
        const albumKey = Object.keys(library.albums).find(k => library.albums[k].tracks.some(t => t.path === contextMenu.tracks[0].path))
        if (albumKey && app.isAlbumSelectedForDap(albumKey)) {
          dapBtn.querySelector('span').textContent = isMulti ? `Remove ${contextMenu.tracks.length} tracks from my DAP` : 'Remove from my DAP'
        } else {
          dapBtn.querySelector('span').textContent = isMulti ? `Add ${contextMenu.tracks.length} tracks to my DAP` : 'Add to my DAP'
        }
      } else {
        dapBtn.querySelector('span').textContent = 'Add to my DAP'
      }
    }
  }
    case 'toggle-dap-track': {
      if (!app.hasDapDestination || !app.hasDapDestination()) {
        showToast('No DAP destination configured')
        break
      }
      // Collect unique album keys from all tracks in context
      const albumKeys = new Set()
      for (const t of contextMenu.tracks) {
        const key = Object.keys(library.albums).find(k => library.albums[k].tracks.some(tr => tr.path === t.path))
        if (key) albumKeys.add(key)
      }
      if (albumKeys.size === 0) break
      app.showDapSyncModal({
        albumKeys: [...albumKeys],
      })
      break
    }

