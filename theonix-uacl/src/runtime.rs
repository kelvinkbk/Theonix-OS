use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{info, warn};

pub struct RuntimeManager {
    base_prefix_dir: PathBuf,
}

impl RuntimeManager {
    pub fn new() -> Result<Self> {
        let base_prefix_dir = dirs::data_local_dir()
            .context("Failed to get local data dir")?
            .join("theonix-uacl")
            .join("prefixes");

        if !base_prefix_dir.exists() {
            std::fs::create_dir_all(&base_prefix_dir)?;
        }

        Ok(Self { base_prefix_dir })
    }

    /// Creates a new isolated WINEPREFIX for the given app id.
    pub fn create_wine_prefix(&self, app_id: &str) -> Result<PathBuf> {
        let prefix_path = self.base_prefix_dir.join(app_id);
        
        if prefix_path.exists() {
            info!("WINEPREFIX already exists at {:?}", prefix_path);
            return Ok(prefix_path);
        }

        info!("Creating new WINEPREFIX at {:?}", prefix_path);
        
        // Execute wineboot to initialize the prefix
        let status = Command::new("wineboot")
            .env("WINEPREFIX", &prefix_path)
            .env("WINEDEBUG", "-all")
            .arg("--init")
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to initialize WINEPREFIX");
        }

        Ok(prefix_path)
    }

    /// Installs a dependency using winetricks in the specified prefix.
    pub fn install_winetrick(&self, prefix_path: &Path, trick: &str) -> Result<()> {
        info!("Installing winetrick '{}' in {:?}", trick, prefix_path);
        
        let status = Command::new("winetricks")
            .env("WINEPREFIX", prefix_path)
            .env("WINEDEBUG", "-all")
            .arg("-q") // quiet/unattended
            .arg(trick)
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to install winetrick: {}", trick);
        }

        Ok(())
    }

    /// Executes a Windows binary within a specific prefix
    pub fn run_executable(&self, prefix_path: &Path, exe_path: &Path, args: &[String]) -> Result<()> {
        info!("Running {:?} in prefix {:?}", exe_path, prefix_path);
        
        let mut cmd = Command::new("wine");
        cmd.env("WINEPREFIX", prefix_path)
           .env("WINEDEBUG", "-all")
           .arg(exe_path);
           
        for arg in args {
            cmd.arg(arg);
        }

        // We use spawn here instead of status if we want it to run in background, 
        // but for now we wait for it to exit or just launch it.
        // In a real launcher, we might detach it.
        let status = cmd.status()?;
        
        if !status.success() {
            warn!("Executable exited with non-zero status");
        }

        Ok(())
    }

    /// Injects DXVK into a WINEPREFIX by downloading and copying the DLLs.
    pub fn install_dxvk(&self, prefix_path: &Path) -> Result<()> {
        info!("Injecting DXVK into {:?}", prefix_path);
        // Note: In production, we would download a specific DXVK release from GitHub
        // For this implementation, we can use winetricks to install dxvk automatically
        self.install_winetrick(prefix_path, "dxvk")?;
        Ok(())
    }

    /// Injects VKD3D into a WINEPREFIX.
    pub fn install_vkd3d(&self, prefix_path: &Path) -> Result<()> {
        info!("Injecting VKD3D into {:?}", prefix_path);
        self.install_winetrick(prefix_path, "vkd3d")?;
        Ok(())
    }
}
