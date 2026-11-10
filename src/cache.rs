use crate::data::{Message, Payload, Section};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
    Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
};
use zeroize::Zeroize;

#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub payload: Payload,
    pub updated: String,
    pub week: String,
    pub page: usize,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Archive {
    pub owner: String,
    pub snapshots: HashMap<Section, Snapshot>,
    pub details: HashMap<String, Message>,
    pub last_section: Section,
}
impl Archive {
    pub fn new(owner: String) -> Self {
        Self {
            owner,
            snapshots: HashMap::new(),
            details: HashMap::new(),
            last_section: Section::Timetable,
        }
    }
}
fn root() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").context("Brak katalogu danych aplikacji")?)
            .join("LibrusLite")
            .join("cache"),
    )
}
fn account_path(owner: &str) -> Result<PathBuf> {
    let hash = Sha256::digest(owner.trim().as_bytes());
    Ok(root()?.join(format!("{hash:x}.bin")))
}

fn protect(input: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    let mut input = input.to_vec();
    let blob = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len())?,
        pbData: input.as_mut_ptr(),
    };
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI binds this data to the current Windows account. No password or
    // encryption key is written next to the cache, and no UI prompt is allowed.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        } else {
            CryptUnprotectData(
                &blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        }
    };
    input.zeroize();
    if ok == 0 {
        return Err(std::io::Error::last_os_error())
            .context("Windows nie mógł zabezpieczyć lub odczytać zapisu");
    }
    let result = if out.cbData == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec()
    };
    unsafe {
        if out.cbData > 0 {
            std::ptr::write_bytes(out.pbData, 0, out.cbData as usize);
        }
        LocalFree(out.pbData.cast());
    }
    Ok(result)
}
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Brak katalogu zapisu")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".write-{}-{}-{}.tmp",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // Atomic replace preserves the old file if writing or renaming fails.
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
fn read_secret(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(protect(&bytes, false)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn load(owner: &str) -> Result<Archive> {
    let Some(mut bytes) = read_secret(&account_path(owner)?)? else {
        return Ok(Archive::new(owner.to_owned()));
    };
    let parsed = serde_json::from_slice::<Archive>(&bytes);
    bytes.zeroize();
    let archive = parsed.context("Nie udało się odczytać ostatniego zapisu")?;
    if archive.owner != owner {
        bail!("Zapis należy do innego konta");
    }
    Ok(archive)
}
pub fn save(archive: &Archive) -> Result<()> {
    let mut bytes = serde_json::to_vec(archive)?;
    let encrypted = protect(&bytes, true);
    bytes.zeroize();
    atomic_write(&account_path(&archive.owner)?, &encrypted?)
}
pub fn activate(owner: &str) -> Result<()> {
    atomic_write(
        &root()?.join("last-account.bin"),
        &protect(owner.as_bytes(), true)?,
    )
}
pub fn last_account() -> Result<Option<String>> {
    let Some(mut bytes) = read_secret(&root()?.join("last-account.bin"))? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes.clone());
    bytes.zeroize();
    Ok(Some(text?))
}
pub fn deactivate() -> Result<()> {
    match fs::remove_file(root()?.join("last-account.bin")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_tabs_survive_encrypted_serialization() {
        let week = crate::data::monday(crate::data::today());
        let mut archive = Archive::new("fictional".into());
        for (section, payload) in crate::demo::payloads(week) {
            archive.snapshots.insert(
                section,
                Snapshot {
                    payload,
                    updated: "10.11.2026 08:00".into(),
                    week: week.to_string(),
                    page: 1,
                },
            );
        }
        let bytes = serde_json::to_vec(&archive).unwrap();
        let restored: Archive =
            serde_json::from_slice(&protect(&protect(&bytes, true).unwrap(), false).unwrap())
                .unwrap();
        assert_eq!(restored.owner, "fictional");
        assert_eq!(restored.snapshots.len(), 5);
        assert_eq!(
            serde_json::to_value(&archive.snapshots).unwrap(),
            serde_json::to_value(restored.snapshots).unwrap()
        );
    }
    #[test]
    fn encrypted_snapshot_roundtrip_and_account_binding() {
        let bytes = b"fictional private school records";
        let encrypted = protect(bytes, true).unwrap();
        assert_ne!(encrypted, bytes);
        assert_eq!(protect(&encrypted, false).unwrap(), bytes);
        let mut damaged = encrypted;
        damaged[0] ^= 0xff;
        assert!(protect(&damaged, false).is_err());
    }
    #[test]
    fn atomic_replace_keeps_the_previous_file_on_failure() {
        let directory = std::env::temp_dir().join(format!(
            "librus-cache-test-{}-{}",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("snapshot.bin");
        atomic_write(&path, b"previous").unwrap();
        // A reader denying deletion prevents replacement, as a disk/permission
        // failure would; the original must remain byte-for-byte unchanged.
        use std::os::windows::fs::OpenOptionsExt;
        let handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(atomic_write(&path, b"new").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        drop(handle);
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
