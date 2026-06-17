use std::fs;
use std::path::Path;
use std::ptr;
use winapi::um::dpapi::{CryptProtectData, CryptUnprotectData};
use winapi::um::wincrypt::CRYPTOAPI_BLOB;
use winapi::um::winbase::LocalFree;

pub fn encrypt_and_write(path: &Path, plaintext: &[u8]) -> Result<(), String> {
    let encrypted = dpapi_encrypt(plaintext)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, &encrypted).map_err(|e| e.to_string())
}

pub fn read_and_decrypt(path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    dpapi_decrypt(&data)
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

fn dpapi_encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input = CRYPTOAPI_BLOB {
        cbData: plaintext.len() as u32,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPTOAPI_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let ok = unsafe {
        CryptProtectData(
            &mut input,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut output,
        )
    };

    if ok == 0 {
        return Err("CryptProtectData failed".into());
    }

    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as *mut _) };
    Ok(result)
}

fn dpapi_decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let mut input = CRYPTOAPI_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPTOAPI_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut output,
        )
    };

    if ok == 0 {
        return Err("CryptUnprotectData failed".into());
    }

    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as *mut _) };
    Ok(result)
}
