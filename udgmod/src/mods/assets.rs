use std::{
    ffi::{CStr, c_char},
    fs,
    path::PathBuf,
};

use walkdir::WalkDir;

use crate::{hack::trampoline, util::init};

init!(
    [
        GET_ASSET_SIZE<fn(*const c_char) -> usize>(0x0010_dda0) => get_asset_size;
        READ_ASSET<fn(*const c_char, *mut u8, usize) -> usize>(0x0010_da20) => read_asset;
    ]
    start: start;
);

const BASE_PATH: &str = "udgmod";

fn start() {
    // Ensure the base path exists
    if let Err(e) = fs::create_dir_all(BASE_PATH) {
        log::warn!("Failed to create base asset directory '{BASE_PATH}': {e}");
        return;
    }

    let mut dds_files = vec![];
    for file in WalkDir::new(BASE_PATH).into_iter().filter_map(|e| {
        e.inspect_err(|e| log::warn!("Error accessing file: {e}"))
            .ok()
    }) {
        if file.file_type().is_file()
            && file
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dds"))
        {
            dds_files.push(file);
        }
    }

    for file in dds_files {
        let Ok(modified) = fs::metadata(file.path())
            .and_then(|meta| meta.modified())
            .inspect_err(|e| {
                log::warn!("Failed to read metadata for {}: {e}", file.path().display());
            })
        else {
            continue;
        };

        let btx = file.path().with_extension("btx");
        if let Ok(btx_modified) = fs::metadata(&btx).and_then(|meta| meta.modified())
            && btx_modified >= modified
        {
            continue; // BTX is up to date
        }

        let Ok(mut data) = fs::read(file.path())
            .inspect_err(|e| log::warn!("Failed to read {}: {e}", file.path().display()))
        else {
            continue;
        };

        // Apply expected prefix for DDS textures
        data.splice(0..0, b"DDS1".iter().copied());

        fs::write(&btx, data)
            .inspect_err(|e| log::warn!("Failed to write {}: {e}", btx.display()))
            .ok();
    }
}

extern "C" fn get_asset_size(entry: *const c_char) -> usize {
    if let Some(path) = file_path(entry)
        && let Ok(metadata) = fs::metadata(&path)
        && let Ok(size) = usize::try_from(metadata.len())
    {
        return size;
    }
    trampoline!(GET_ASSET_SIZE(entry))
}

extern "C" fn read_asset(entry: *const c_char, buffer: *mut u8, size: usize) -> usize {
    if let Some(path) = file_path(entry)
        && let Ok(bytes) = fs::read(&path)
        && bytes.len() == size
    {
        log::info!("Loading asset from disk: {}", path.display());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, size);
        }
        return size;
    }
    trampoline!(READ_ASSET(entry, buffer, size))
}

fn file_path(entry: *const c_char) -> Option<PathBuf> {
    let c_str = unsafe { CStr::from_ptr(entry.as_ref()?) };
    let str_slice = c_str.to_str().ok()?;
    Some(PathBuf::from("udgmod").join(str_slice))
}
