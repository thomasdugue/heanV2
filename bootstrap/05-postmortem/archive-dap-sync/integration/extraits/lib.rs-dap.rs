// Code DAP Sync retiré de lib.rs par le commit 47acae0 (2026-04-22) — 22 commandes Tauri + types + state.

// === DAP SYNC (SD Card / USB sync) ===
mod dap_sync;

        let local_path = dap_sync::smb_utils::resolve_smb_path(
            &dap_sync::smb_utils::build_smb_mount_map(),
        let local_path = dap_sync::smb_utils::resolve_smb_path(
            &dap_sync::smb_utils::build_smb_mount_map(),
// === DAP SYNC — Tauri Commands ===

/// Global cancel flag for DAP sync operations.
static DAP_SYNC_CANCEL: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

#[tauri::command]
fn dap_list_external_volumes() -> Result<Vec<dap_sync::volumes::ExternalVolume>, String> {
    dap_sync::volumes::list_external_volumes()
}

#[tauri::command]
fn dap_get_volume_info(path: String) -> Result<dap_sync::volumes::VolumeInfo, String> {
    dap_sync::volumes::get_volume_info(&path)
}

#[tauri::command]
fn dap_save_destination(dest: dap_sync::db::DapDestination) -> Result<i64, String> {
    dap_sync::db::save_destination(&dest)
}

#[tauri::command]
fn dap_get_destinations() -> Result<Vec<dap_sync::db::DapDestination>, String> {
    dap_sync::db::get_destinations()
}

#[tauri::command]
fn dap_get_destination(id: i64) -> Result<Option<dap_sync::db::DapDestination>, String> {
    dap_sync::db::get_destination(id)
}

#[tauri::command]
fn dap_delete_destination(id: i64) -> Result<(), String> {
    dap_sync::db::delete_destination(id)
}

#[tauri::command]
fn dap_eject_volume(path: String) -> Result<(), String> {
    use std::process::Command;
    let output = Command::new("diskutil")
        .args(["eject", &path])
        .output()
        .map_err(|e| format!("Failed to run diskutil: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Eject failed: {}", stderr.trim()))
    }
}

#[tauri::command]
fn dap_save_selection(destination_id: i64, album_id: i64, selected: bool) -> Result<(), String> {
    dap_sync::db::save_selection(destination_id, album_id, selected)
}

#[tauri::command]
fn dap_save_selections_batch(destination_id: i64, selections: Vec<(i64, bool)>) -> Result<(), String> {
    dap_sync::db::save_selections_batch(destination_id, &selections)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DapSelection {
    album_id: i64,
    selected: bool,
}

#[tauri::command]
fn dap_get_selections(destination_id: i64) -> Result<Vec<DapSelection>, String> {
    let sels = dap_sync::db::get_selections(destination_id)?;
    Ok(sels.into_iter().map(|(album_id, selected)| DapSelection { album_id, selected }).collect())
}

#[tauri::command]
fn dap_read_manifest(dest_path: String) -> Result<Option<dap_sync::manifest::SyncManifest>, String> {
    dap_sync::manifest::read_manifest(&dest_path)
}

#[tauri::command]
async fn dap_compute_sync_plan(
    tracks: Vec<dap_sync::sync_plan::TrackForSync>,
    dest_path: String,
    folder_structure: String,
    mirror_mode: bool,
) -> Result<dap_sync::sync_plan::SyncPlan, String> {
    let cmd_start = std::time::Instant::now();
    #[cfg(debug_assertions)]
    eprintln!("[PERF-RS] dap_compute_sync_plan called with {} tracks, dest={}", tracks.len(), dest_path);

    let is_mtp = dest_path.starts_with("mtp://");

    // Skip filesystem migration for MTP — no local filesystem to migrate
    if !is_mtp {
        let migrated = dap_sync::sync_plan::migrate_dap_filesystem(&dest_path);
        if migrated > 0 {
            eprintln!("[DAP-SYNC] Migrated {} manifest entries to match current sanitization rules", migrated);
        }
    }

    let t0 = std::time::Instant::now();
    // For MTP, manifest is stored locally (not on device)
    let manifest = if is_mtp {
        let mtp_path = dap_sync::manifest::mtp_manifest_path(&dest_path);
        dap_sync::manifest::read_manifest_file(&mtp_path)?
    } else {
        dap_sync::manifest::read_manifest(&dest_path)?
    };
    #[cfg(debug_assertions)]
    eprintln!("[PERF-RS] read_manifest: {:?} ({} files)", t0.elapsed(),
        manifest.as_ref().map(|m| m.files.len()).unwrap_or(0));

    // For MTP, scan actual device files to validate manifest against reality.
    // Without this, a formatted SD card would still show "ON DAP" for all albums
    // because the local manifest has no way to know the device was wiped.
    let mtp_device_files = if is_mtp {
        let parts: Vec<&str> = dest_path.trim_start_matches("mtp://").split('/').collect();
        let storage_idx: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        let t_mtp = std::time::Instant::now();
        match dap_sync::mtp::scan_mtp_device_files(storage_idx).await {
            Ok(files) => {
                #[cfg(debug_assertions)]
                eprintln!("[PERF-RS] MTP device scan: {:?} ({} files)", t_mtp.elapsed(), files.len());
                // Clean the local manifest: remove entries for files no longer on the device.
                // This handles ghost directories (folder created but files never written).
                if let Err(e) = dap_sync::mtp::clean_mtp_manifest(&dest_path, &files) {
                    eprintln!("[MTP] Warning: manifest cleanup failed: {}", e);
                }
                Some(files)
            }
            Err(e) => {
                eprintln!("[MTP] Warning: device scan failed ({}), falling back to manifest only", e);
                None
            }
        }
    } else {
        None
    };

    let t1 = std::time::Instant::now();
    // For MTP, get volume info from the cached MTP detection (not filesystem df)
    let vol_info = if is_mtp {
        // Parse storage index from mtp://serial/index
        let parts: Vec<&str> = dest_path.trim_start_matches("mtp://").split('/').collect();
        let _storage_idx: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        // Use a large default — actual free space was shown in sidebar from detect_mtp_devices
        dap_sync::volumes::VolumeInfo {
            name: "MTP Device".into(),
            path: dest_path.clone(),
            free_bytes: 2_000_000_000_000, // 2TB default for MTP
            total_bytes: 2_147_000_000_000,
            is_mounted: true,
        }
    } else {
        dap_sync::volumes::get_volume_info(&dest_path)?
    };
    #[cfg(debug_assertions)]
    eprintln!("[PERF-RS] get_volume_info: {:?}", t1.elapsed());

    // Clone cover cache entries (brief lock) for cover art resolution
    let cover_entries = COVER_CACHE.lock()
        .map(|c| c.entries.clone())
        .unwrap_or_default();
    let covers_dir = get_cover_cache_dir();

    let result = dap_sync::sync_plan::compute_sync_plan(
        &tracks,
        &manifest,
        &folder_structure,
        mirror_mode,
        mtp_device_files,
        vol_info.free_bytes,
        vol_info.total_bytes,
        &cover_entries,
        &covers_dir,
        &dest_path,
    );
    #[cfg(debug_assertions)]
    eprintln!("[PERF-RS] dap_compute_sync_plan TOTAL: {:?}", cmd_start.elapsed());
    Ok(result)
}

#[tauri::command]
fn dap_execute_sync(
    app_handle: tauri::AppHandle,
    tracks: Vec<dap_sync::sync_plan::TrackForSync>,
    dest_path: String,
    folder_structure: String,
    mirror_mode: bool,
) {
    use tauri::Emitter;

    // Reset cancel flag
    DAP_SYNC_CANCEL.store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel_flag = DAP_SYNC_CANCEL.clone();

    std::thread::spawn(move || {
        // Compute plan fresh
        let manifest = match if dest_path.starts_with("mtp://") {
            // MTP: no manifest on device — start fresh (manifest will be stored locally later)
            Ok(None)
        } else {
            dap_sync::manifest::read_manifest(&dest_path)
        } {
            Ok(m) => m,
            Err(e) => {
                let _ = app_handle.emit("dap_sync_complete", dap_sync::sync_engine::SyncComplete {
                    success: false,
                    files_copied: 0,
                    files_deleted: 0,
                    total_bytes_copied: 0,
                    duration_ms: 0,
                    errors: vec![format!("Failed to read manifest: {}", e)],
                });
                return;
            }
        };

        let is_mtp = dest_path.starts_with("mtp://");
        let vol_info = if is_mtp {
            // MTP: use large defaults (actual space checked by device firmware)
            Ok(dap_sync::volumes::VolumeInfo {
                name: "MTP Device".into(),
                path: dest_path.clone(),
                free_bytes: 2_000_000_000_000,
                total_bytes: 2_147_000_000_000,
                is_mounted: true,
            })
        } else {
            dap_sync::volumes::get_volume_info(&dest_path)
        };
        let vol_info = match vol_info {
            Ok(v) => v,
            Err(e) => {
                let _ = app_handle.emit("dap_sync_complete", dap_sync::sync_engine::SyncComplete {
                    success: false,
                    files_copied: 0,
                    files_deleted: 0,
                    total_bytes_copied: 0,
                    duration_ms: 0,
                    errors: vec![format!("Failed to get volume info: {}", e)],
                });
                return;
            }
        };

        // Clone cover cache entries (brief lock) for cover art resolution
        let cover_entries = COVER_CACHE.lock()
            .map(|c| c.entries.clone())
            .unwrap_or_default();
        let covers_dir = get_cover_cache_dir();

        let plan = dap_sync::sync_plan::compute_sync_plan(
            &tracks,
            &manifest,
            &folder_structure,
            mirror_mode,
            None, // No external device files for regular volume sync
            vol_info.free_bytes,
            vol_info.total_bytes,
            &cover_entries,
            &covers_dir,
            &dest_path,
        );

        if !plan.enough_space {
            let _ = app_handle.emit("dap_sync_complete", dap_sync::sync_engine::SyncComplete {
                success: false,
                files_copied: 0,
                files_deleted: 0,
                total_bytes_copied: 0,
                duration_ms: 0,
                errors: vec!["Not enough space on destination".into()],
            });
            return;
        }

        // Build network source list for UUID → hostname SMB resolution
        let net_sources: Vec<(String, String)> = NETWORK_SOURCES
            .lock()
            .map(|sources| sources.iter().map(|s| (s.id.clone(), s.host.clone())).collect())
            .unwrap_or_default();

        match dap_sync::sync_engine::execute_sync(
            &app_handle,
            &dest_path,
            &plan,
            &manifest,
            &folder_structure,
            cancel_flag,
            net_sources,
        ) {
            Ok(_) => {} // SyncComplete already emitted by execute_sync
            Err(e) => {
                let _ = app_handle.emit("dap_sync_complete", dap_sync::sync_engine::SyncComplete {
                    success: false,
                    files_copied: 0,
                    files_deleted: 0,
                    total_bytes_copied: 0,
                    duration_ms: 0,
                    errors: vec![e],
                });
            }
        }
    });
}

#[tauri::command]
async fn dap_execute_mtp_sync(
    app_handle: tauri::AppHandle,
    files: Vec<(String, String)>,          // (source_path, dest_relative_path) — files to copy
    files_to_delete: Vec<String>,          // dest_relative_paths to delete from device
    storage_index: usize,
    dest_path: String,                     // mtp://serial/index — for manifest storage
    folder_structure: String,
) -> Result<(), String> {
    use tauri::Emitter;

    let start = std::time::Instant::now();
    let total_copy = files.len();
    let total_delete = files_to_delete.len();
    eprintln!("[MTP-SYNC] Starting MTP sync: {} files to copy, {} to delete on storage {}",
        total_copy, total_delete, storage_index);

    // Resolve SMB paths before passing to MTP
    let smb_map = dap_sync::smb_utils::build_smb_mount_map();
    let resolved_files: Vec<(String, String)> = files.iter().map(|(src, dest)| {
        let resolved = dap_sync::smb_utils::resolve_smb_path(src, &smb_map);
        (resolved, dest.clone())
    }).collect();
    // Keep ORIGINAL (non-resolved) source paths for manifest
    // (resolved paths are session-specific /Volumes/... mounts that change between reboots;
    //  original paths are stable smb:// URIs that match library.tracks[].path in JS)
    let original_files_for_manifest: Vec<(String, String)> = files.into_iter().collect();

    // Reset cancel flag
    DAP_SYNC_CANCEL.store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel_flag = DAP_SYNC_CANCEL.clone();

    let mut all_errors: Vec<String> = Vec::new();
    let mut total_files_deleted: usize = 0;

    // --- Phase 0: Purge ghost (empty) folders on the device ---
    // Ghost folders are created when MTP uploads fail mid-way: the folder exists
    // but contains no files. The JM21 firmware ignores them but they waste space
    // and confuse the sync plan.
    match dap_sync::mtp::mtp_purge_empty_folders(storage_index).await {
        Ok(count) if count > 0 => {
            eprintln!("[MTP-SYNC] Pre-sync ghost purge: {} empty folders deleted", count);
        }
        Ok(_) => {} // no ghosts found
        Err(e) => {
            eprintln!("[MTP-SYNC] Ghost purge failed (non-fatal): {}", e);
        }
    }

    // --- Phase 1: Delete files no longer in selection ---
    if !files_to_delete.is_empty() {
        let app_handle_del = app_handle.clone();
        let cancel_del = cancel_flag.clone();

        match dap_sync::mtp::mtp_delete_files(
            &files_to_delete,
            storage_index,
            move |current, total, file| {
                let _ = app_handle_del.emit("dap_sync_progress", serde_json::json!({
                    "phase": "delete",
                    "current": current,
                    "total": total,
                    "currentFile": file,
                    "bytesCopied": 0,
                    "totalBytes": 0,
                    "action": "delete"
                }));
            },
            cancel_del,
        ).await {
            Ok((deleted, errors)) => {
                total_files_deleted = deleted;
                eprintln!("[MTP-SYNC] Delete phase: {}/{} deleted, {} errors",
                    deleted, total_delete, errors.len());
                all_errors.extend(errors);
            }
            Err(e) => {
                eprintln!("[MTP-SYNC] Delete phase failed: {}", e);
                all_errors.push(format!("Delete phase failed: {}", e));
            }
        }
    }

    // --- Phase 2: Copy new files ---
    let app_handle_copy = app_handle.clone();
    let cancel_copy = cancel_flag.clone();

    let app_handle_replug = app_handle.clone();
    let result = dap_sync::mtp::mtp_sync_batch(
        resolved_files.clone(),
        storage_index,
        move |current, total, file| {
            let _ = app_handle_copy.emit("dap_sync_progress", serde_json::json!({
                "phase": "copy",
                "current": current,
                "total": total,
                "currentFile": file,
                "bytesCopied": 0,
                "totalBytes": 0,
                "action": "copy"
            }));
        },
        cancel_copy,
        Some(app_handle_replug),
    ).await;

    let (files_copied, total_bytes_copied) = match result {
        Ok((copied, bytes, errors)) => {
            all_errors.extend(errors);
            (copied, bytes)
        }
        Err(e) => {
            all_errors.push(e);
            (0, 0)
        }
    };

    // --- Phase 3: Write manifest locally ---
    // Build manifest from the files that should now be on the device:
    // = files we just copied + files that were already on device (unchanged from plan)
    let manifest_path = dap_sync::manifest::mtp_manifest_path(&dest_path);
    let old_manifest = dap_sync::manifest::read_manifest_file(&manifest_path)
        .unwrap_or(None);

    // Start with unchanged files from old manifest (not deleted)
    let deleted_set: std::collections::HashSet<String> = files_to_delete.iter().cloned().collect();
    let mut manifest_files: Vec<dap_sync::manifest::SyncedFile> = Vec::new();

    if let Some(old) = &old_manifest {
        for f in &old.files {
            if !deleted_set.contains(&f.dest_relative_path) {
                manifest_files.push(f.clone());
            }
        }
    }

    // Add newly copied files — exclude files that failed (in error list).
    // Build a set of dest_relative_paths that had errors to exclude from manifest.
    let error_dest_paths: std::collections::HashSet<String> = all_errors.iter()
        .filter_map(|e| {
            // Error format: "dest_rel — error message"
            e.split(" — ").next().map(|s| s.to_string())
        })
        .collect();

    let existing_dest_paths: std::collections::HashSet<String> =
        manifest_files.iter().map(|f| f.dest_relative_path.clone()).collect();

    // Build resolved source map for size lookup (original path → resolved path)
    let resolved_map: std::collections::HashMap<&str, &str> = original_files_for_manifest.iter()
        .zip(resolved_files.iter())
        .map(|((orig, _), (resolved, _))| (orig.as_str(), resolved.as_str()))
        .collect();

    for (original_src, dest_rel) in &original_files_for_manifest {
        if !existing_dest_paths.contains(dest_rel) && !error_dest_paths.contains(dest_rel) {
            let fallback = original_src.as_str();
            let resolved_src = resolved_map.get(original_src.as_str()).unwrap_or(&fallback);
            manifest_files.push(dap_sync::manifest::SyncedFile {
                source_path: original_src.clone(),  // Store ORIGINAL smb:// path, not resolved /Volumes/
                dest_relative_path: dest_rel.clone(),
                size_bytes: std::fs::metadata(resolved_src).map(|m| m.len()).unwrap_or(0),
                modified_at: String::new(),
                quick_hash: String::new(),
            });
        }
    }

    let now = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}", secs)
    };

    let new_manifest = dap_sync::manifest::SyncManifest {
        hean_version: "1.0.0".into(),
        last_sync: now,
        destination_path: dest_path.clone(),
        folder_structure,
        files: manifest_files,
    };

    if let Err(e) = dap_sync::manifest::write_manifest_file(&manifest_path, &new_manifest) {
        eprintln!("[MTP-SYNC] Failed to write manifest: {}", e);
        all_errors.push(format!("Manifest write failed: {}", e));
    } else {
        eprintln!("[MTP-SYNC] Manifest written: {} files tracked at {}",
            new_manifest.files.len(), manifest_path);
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let success = all_errors.is_empty();

    eprintln!("[MTP-SYNC] === SYNC {} === {} copied, {} deleted, {:.1} GB in {:.1}s, {} errors",
        if success { "COMPLETE" } else { "FINISHED WITH ERRORS" },
        files_copied, total_files_deleted,
        total_bytes_copied as f64 / 1_073_741_824.0,
        duration_ms as f64 / 1000.0, all_errors.len());

    let _ = app_handle.emit("dap_sync_complete", dap_sync::sync_engine::SyncComplete {
        success,
        files_copied,
        files_deleted: total_files_deleted,
        total_bytes_copied,
        duration_ms,
        errors: all_errors,
    });

    Ok(())
}

#[tauri::command]
fn dap_cancel_sync() {
    eprintln!("[DAP-SYNC] *** CANCEL REQUESTED BY USER ***");
    DAP_SYNC_CANCEL.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
fn dap_start_volume_watcher(app_handle: tauri::AppHandle) {
    dap_sync::watcher::start_volume_watcher(app_handle);
}

#[tauri::command]
async fn dap_detect_mtp_devices() -> Result<Vec<dap_sync::mtp::MtpDeviceInfo>, String> {
    dap_sync::mtp::detect_mtp_devices().await
}

#[tauri::command]
async fn dap_mtp_send_file(source_path: String, dest_folder: String, dest_filename: String, storage_index: usize) -> Result<u64, String> {
    dap_sync::mtp::mtp_send_file(&source_path, &dest_folder, &dest_filename, storage_index).await
}

#[tauri::command]
async fn dap_mtp_list_files(storage_index: usize) -> Result<Vec<dap_sync::mtp::MtpFileEntry>, String> {
    dap_sync::mtp::mtp_list_files(storage_index).await
}

#[tauri::command]
async fn dap_mtp_test(source_path: String) -> Result<String, String> {
    dap_sync::mtp::mtp_test_all(&source_path).await
}

#[tauri::command]
async fn dap_mtp_purge_albums(folder_names: Vec<String>, storage_index: usize) -> Result<(usize, Vec<String>), String> {
    dap_sync::mtp::mtp_purge_album_folders(folder_names, storage_index).await
}

#[tauri::command]
async fn dap_mtp_purge_ghosts(storage_index: usize) -> Result<usize, String> {
    dap_sync::mtp::mtp_purge_empty_folders(storage_index).await
}

            media_controls::init_media_controls(app_handle.clone());

            // Lance le watcher de volumes pour détecter montage/démontage USB/SD
            dap_sync::watcher::start_volume_watcher(app_handle);
            // DAP Sync (SD Card / USB sync)
            dap_list_external_volumes,
            dap_get_volume_info,
            dap_save_destination,
            dap_get_destinations,
            dap_get_destination,
            dap_delete_destination,
            dap_eject_volume,
            dap_save_selection,
            dap_save_selections_batch,
            dap_get_selections,
            dap_read_manifest,
            dap_compute_sync_plan,
            dap_execute_sync,
            dap_cancel_sync,
            dap_start_volume_watcher,
            dap_detect_mtp_devices,
            dap_execute_mtp_sync,
            dap_mtp_send_file,
            dap_mtp_list_files,
            dap_mtp_test,
            dap_mtp_purge_albums,
            dap_mtp_purge_ghosts,
