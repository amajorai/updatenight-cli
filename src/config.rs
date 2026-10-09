use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub token: Option<String>,
}

fn config_path() -> anyhow::Result<PathBuf> {
    let root = dirs::config_dir()
        .filter(|path| path.is_absolute())
        .ok_or_else(|| anyhow::anyhow!("User configuration directory unavailable"))?;
    Ok(root.join("updatenight").join("config.json"))
}

fn save_to(path: &std::path::Path, config: &Config) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing config parent"))?;
    let mut directory = std::fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(parent)?;
    let metadata = std::fs::symlink_metadata(parent)?;
    anyhow::ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "Unsafe config directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    private_permissions(parent)?;
    let temporary = parent.join(format!(
        ".credential-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> anyhow::Result<()> {
        let mut file = options.open(&temporary)?;
        private_permissions(&temporary)?;
        file.write_all(serde_json::to_string_pretty(config)?.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn load_from(path: &std::path::Path) -> anyhow::Result<Config> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing config parent"))?;
    let parent_metadata = fs::symlink_metadata(parent)?;
    anyhow::ensure!(
        parent_metadata.is_dir() && !parent_metadata.file_type().is_symlink(),
        "Unsafe config directory"
    );
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "Unsafe credential file"
    );
    private_permissions(parent)?;
    private_permissions(path)?;
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn load() -> Config {
    config_path()
        .and_then(|path| load_from(&path))
        .unwrap_or_default()
}

#[cfg(unix)]
fn private_permissions(path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if path.is_dir() { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(windows)]
fn private_permissions(path: &std::path::Path) -> anyhow::Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE},
        Security::Authorization::ConvertSidToStringSidW,
        Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    let sid = unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        anyhow::ensure!(
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0,
            "Cannot identify credential owner"
        );
        let result = (|| -> anyhow::Result<String> {
            let mut size = 0;
            GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
            let mut storage = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
            anyhow::ensure!(
                GetTokenInformation(
                    token,
                    TokenUser,
                    storage.as_mut_ptr().cast(),
                    size,
                    &mut size
                ) != 0,
                "Cannot read credential owner"
            );
            let user = &*storage.as_ptr().cast::<TOKEN_USER>();
            let mut text = std::ptr::null_mut();
            anyhow::ensure!(
                ConvertSidToStringSidW(user.User.Sid, &mut text) != 0,
                "Cannot resolve credential owner"
            );
            let mut length = 0;
            while *text.add(length) != 0 {
                length += 1;
            }
            let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, length));
            LocalFree(text.cast());
            Ok(sid)
        })();
        CloseHandle(token);
        result?
    };
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::{
        SetFileSecurityW, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    };
    let descriptor_text: Vec<u16> = format!("D:P(A;;FA;;;{sid})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let mut descriptor = std::ptr::null_mut();
        anyhow::ensure!(
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor_text.as_ptr(),
                1,
                &mut descriptor,
                std::ptr::null_mut()
            ) != 0,
            "Cannot prepare private credential permissions"
        );
        let success = SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        );
        LocalFree(descriptor.cast());
        anyhow::ensure!(success != 0, "Cannot protect credential permissions");
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn private_permissions(_: &std::path::Path) -> anyhow::Result<()> {
    anyhow::bail!("Credential storage is unsupported on this platform")
}

pub fn save(cfg: &Config) -> anyhow::Result<()> {
    save_to(&config_path()?, cfg)
}

pub fn logout() -> Result<()> {
    let mut cfg = load();
    cfg.token = None;
    save(&cfg)?;
    println!("Logged out.");
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn credentials_are_private_and_replace_destination_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "credential-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("app/config.json");
        save_to(&path, &Config::default()).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        load_from(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let victim = root.join("victim");
        std::fs::write(&victim, "keep").unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(&victim, &path).unwrap();
        assert!(load_from(&path).is_err());
        save_to(&path, &Config::default()).unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
        std::fs::remove_dir_all(root).unwrap();
    }
}
