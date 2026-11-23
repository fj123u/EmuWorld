//! Secrets on disk (iiDB / SteamGridDB keys, RetroAchievements, B2 backups):
//! encrypted so the files are useless copied elsewhere.
//!
//! - Windows: DPAPI (`CryptProtectData`), tied to the Windows user.
//! - Linux / other: AES-256-GCM with a random key kept in
//!   `<data dir>/.secret_key`, readable by the user only (0600). Same
//!   protection level as DPAPI for a copy of the data files; the key file
//!   stays on this machine.
//!
//! Same two functions everywhere: `encrypt_and_write`, `read_and_decrypt`.

use std::fs;
use std::path::Path;

pub fn encrypt_and_write(path: &Path, plaintext: &[u8]) -> Result<(), String> {
    let encrypted = platform::encrypt(plaintext)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    crate::atomic_file::write(path, &encrypted).map_err(|e| e.to_string())
}

pub fn read_and_decrypt(path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    platform::decrypt(&data)
}

pub fn migrate_plaintext_if_needed(path: &Path) {
    if !path.exists() {
        return;
    }
    if let Ok(data) = fs::read(path) {
        if data.starts_with(b"{") {
            if let Ok(()) = encrypt_and_write(path, &data) {
                // migrated
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::ptr;
    use winapi::um::dpapi::{CryptProtectData, CryptUnprotectData};
    use winapi::um::winbase::LocalFree;
    use winapi::um::wincrypt::CRYPTOAPI_BLOB;

    pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut input = CRYPTOAPI_BLOB { cbData: plaintext.len() as u32, pbData: plaintext.as_ptr() as *mut u8 };
        let mut output = CRYPTOAPI_BLOB { cbData: 0, pbData: ptr::null_mut() };
        let ok = unsafe {
            CryptProtectData(&mut input, ptr::null(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), 0, &mut output)
        };
        if ok == 0 {
            return Err("CryptProtectData failed".into());
        }
        let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe { LocalFree(output.pbData as *mut _) };
        Ok(result)
    }

    pub fn decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let mut input = CRYPTOAPI_BLOB { cbData: ciphertext.len() as u32, pbData: ciphertext.as_ptr() as *mut u8 };
        let mut output = CRYPTOAPI_BLOB { cbData: 0, pbData: ptr::null_mut() };
        let ok = unsafe {
            CryptUnprotectData(&mut input, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), 0, &mut output)
        };
        if ok == 0 {
            return Err("CryptUnprotectData failed".into());
        }
        let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe { LocalFree(output.pbData as *mut _) };
        Ok(result)
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
        super::sealed::seal(&super::sealed::machine_key()?, plaintext)
    }

    pub fn decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        super::sealed::open(&super::sealed::machine_key()?, ciphertext)
    }
}

/// AES-256-GCM sealing used off Windows (and tested everywhere).
#[cfg_attr(windows, allow(dead_code))]
mod sealed {
    use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use std::path::Path;

    /// Format: magic, 12-byte nonce, ciphertext + tag.
    const MAGIC: &[u8; 4] = b"EWS1";

    pub fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let body = cipher.encrypt(&nonce, plaintext).map_err(|_| "chiffrement impossible".to_string())?;
        let mut out = Vec::with_capacity(4 + 12 + body.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&body);
        Ok(out)
    }

    pub fn open(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 4 + 12 + 16 || &data[..4] != MAGIC {
            return Err("fichier chiffré illisible".to_string());
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        cipher.decrypt(Nonce::from_slice(&data[4..16]), &data[16..]).map_err(|_| "déchiffrement impossible (autre machine ?)".to_string())
    }

    /// The key in `path`, created (32 random bytes, owner-only) the first time.
    pub fn key_at(path: &Path) -> Result<[u8; 32], String> {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
                return Ok(key);
            }
            crate::push_log("WARN", "Clé de chiffrement locale invalide, une nouvelle est créée (les clés enregistrées sont à refaire)");
        }
        let key: [u8; 32] = Aes256Gcm::generate_key(&mut OsRng).into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        write_private(path, &key)?;
        crate::push_log("INFO", "Clé de chiffrement locale créée");
        Ok(key)
    }

    #[cfg(unix)]
    fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())
    }

    #[cfg(not(unix))]
    fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    pub fn machine_key() -> Result<[u8; 32], String> {
        key_at(&crate::emuworld_base_dir().join(".secret_key"))
    }
}

#[cfg(test)]
mod tests {
    use super::sealed;

    #[test]
    fn sealed_secrets_round_trip_and_need_the_same_key() {
        let dir = std::env::temp_dir().join(format!("emuworld_sealed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let key = sealed::key_at(&dir.join(".secret_key")).unwrap();
        assert_eq!(sealed::key_at(&dir.join(".secret_key")).unwrap(), key, "the key is kept");
        let sealed_bytes = sealed::seal(&key, b"{\"api_key\":\"abc\"}").unwrap();
        assert!(!sealed_bytes.windows(3).any(|w| w == b"abc"), "never in clear");
        assert_eq!(sealed::open(&key, &sealed_bytes).unwrap(), b"{\"api_key\":\"abc\"}");
        assert!(sealed::open(&[7u8; 32], &sealed_bytes).is_err(), "another machine's key cannot read it");
        assert!(sealed::open(&key, b"{plain json}").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(".secret_key")).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "owner only");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
