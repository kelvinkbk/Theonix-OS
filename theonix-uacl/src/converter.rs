use anyhow::Result;
use std::path::Path;
use std::process::Command;
use log::{info, warn};

pub struct PackageConverter;

impl PackageConverter {
    /// Uses debtap to convert a .deb file to an Arch package and install it via pacman
    pub fn install_deb<P: AsRef<Path>>(deb_path: P) -> Result<()> {
        let deb_path = deb_path.as_ref();
        info!("Converting Debian package: {:?}", deb_path);
        
        // Ensure debtap is updated (could be cached in production)
        // Command::new("debtap").arg("-u").status()?;
        
        let status = Command::new("debtap")
            .arg("-q") // quiet
            .arg(deb_path)
            .status()?;
            
        if !status.success() {
            anyhow::bail!("Debtap conversion failed");
        }
        
        // Find the generated .pkg.tar.zst file
        // This is a naive implementation; in production we'd parse the output or scan the dir
        let file_stem = deb_path.file_stem().unwrap().to_string_lossy();
        let current_dir = std::env::current_dir()?;
        
        for entry in std::fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "zst" && path.to_string_lossy().contains(&*file_stem) {
                        info!("Installing converted package: {:?}", path);
                        let pacman_status = Command::new("sudo")
                            .arg("pacman")
                            .arg("-U")
                            .arg("--noconfirm")
                            .arg(&path)
                            .status()?;
                            
                        if !pacman_status.success() {
                            anyhow::bail!("Failed to install converted package");
                        }
                        
                        // Cleanup
                        let _ = std::fs::remove_file(path);
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Handles AppImage integration (make executable and run or integrate with appimaged)
    pub fn launch_appimage<P: AsRef<Path>>(appimage_path: P) -> Result<()> {
        let path = appimage_path.as_ref();
        info!("Launching AppImage: {:?}", path);
        
        // Ensure it's executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
        
        // Launch it
        Command::new(path)
            .spawn()?; // Spawn so it detaches from our launcher
            
        Ok(())
    }
}
