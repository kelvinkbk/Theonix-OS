use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{info, warn};

/// Runtime profiles define how Wine is configured for different app types
#[derive(Debug, Clone)]
pub enum RuntimeProfile {
    Gaming,      // Wine-GE or Proton, DXVK enabled, Windows 10
    Office,      // Wine Stable, no DXVK, Windows 7
    Legacy,      // Wine Stable, no DXVK, Windows XP
    Development, // Wine Staging, no DXVK, Windows 10
    Portable,    // Wine Stable, no DXVK, Windows 10 (no install)
    Auto,        // Detect from executable
}

impl RuntimeProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gaming"      => RuntimeProfile::Gaming,
            "office"      => RuntimeProfile::Office,
            "legacy"      => RuntimeProfile::Legacy,
            "development" => RuntimeProfile::Development,
            "portable"    => RuntimeProfile::Portable,
            _             => RuntimeProfile::Auto,
        }
    }

    pub fn windows_version(&self) -> &str {
        match self {
            RuntimeProfile::Legacy      => "winxp",
            RuntimeProfile::Office      => "win7",
            RuntimeProfile::Gaming |
            RuntimeProfile::Development |
            RuntimeProfile::Portable |
            RuntimeProfile::Auto        => "win10",
        }
    }

    pub fn uses_dxvk(&self) -> bool {
        matches!(self, RuntimeProfile::Gaming)
    }

    pub fn name(&self) -> &str {
        match self {
            RuntimeProfile::Gaming      => "gaming",
            RuntimeProfile::Office      => "office",
            RuntimeProfile::Legacy      => "legacy",
            RuntimeProfile::Development => "development",
            RuntimeProfile::Portable    => "portable",
            RuntimeProfile::Auto        => "auto",
        }
    }
}

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

        let status = Command::new("wineboot")
            .env("WINEPREFIX", &prefix_path)
            .env("WINEDEBUG", "-all")
            .env("WINE_HIDE_CRASH_DIALOG", "1")
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
            .arg("--init")
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()?;

        if !status.success() {
            // Non-fatal: prefix may still be usable
            warn!("wineboot returned non-zero, continuing anyway");
        }

        Ok(prefix_path)
    }

    /// Configure the WINEPREFIX Windows version based on the runtime profile
    pub fn apply_profile(&self, prefix_path: &Path, profile: &RuntimeProfile) -> Result<()> {
        let win_ver = profile.windows_version();
        info!("Setting Windows version to '{}' for profile '{}'", win_ver, profile.name());

        Command::new("winetricks")
            .env("WINEPREFIX", prefix_path)
            .env("WINEDEBUG", "-all")
            .env("WINE_HIDE_CRASH_DIALOG", "1")
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
            .args(["-q", win_ver])
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()?;

        Ok(())
    }

    /// Auto-detect and silently install missing dependencies from the detected list
    pub fn auto_install_dependencies(
        &self,
        prefix_path: &Path,
        components: &[&str],
    ) -> Result<()> {
        if components.is_empty() {
            return Ok(());
        }

        info!("Auto-installing {} missing dependencies: {:?}", components.len(), components);

        let mut args = vec!["-q".to_string()];
        args.extend(components.iter().map(|c| c.to_string()));

        let status = Command::new("winetricks")
            .env("WINEPREFIX", prefix_path)
            .env("WINEDEBUG", "-all")
            .env("WINE_HIDE_CRASH_DIALOG", "1")
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .args(&args)
            .status()?;

        if !status.success() {
            warn!("Some dependencies may not have installed correctly");
        }

        Ok(())
    }

    /// Installs a single winetricks component
    pub fn install_winetrick(&self, prefix_path: &Path, trick: &str) -> Result<()> {
        info!("Installing winetrick '{}' in {:?}", trick, prefix_path);

        Command::new("winetricks")
            .env("WINEPREFIX", prefix_path)
            .env("WINEDEBUG", "-all")
            .args(["-q", trick])
            .status()?;

        Ok(())
    }

    /// Executes a Windows binary within a specific prefix
    pub fn run_executable(&self, prefix_path: &Path, exe_path: &Path, args: &[String]) -> Result<()> {
        info!("Running {:?} in prefix {:?}", exe_path, prefix_path);

        let mut cmd = Command::new("wine");
        cmd.env("WINEPREFIX", prefix_path)
           .env("WINEDEBUG", "-all")
           .env("WINE_HIDE_CRASH_DIALOG", "1")
           .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
           .arg(exe_path);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.status()?;

        Ok(())
    }

    /// Injects DXVK into a WINEPREFIX
    pub fn install_dxvk(&self, prefix_path: &Path) -> Result<()> {
        info!("Injecting DXVK into {:?}", prefix_path);
        self.install_winetrick(prefix_path, "dxvk")?;
        Ok(())
    }

    /// Injects VKD3D into a WINEPREFIX
    pub fn install_vkd3d(&self, prefix_path: &Path) -> Result<()> {
        info!("Injecting VKD3D into {:?}", prefix_path);
        self.install_winetrick(prefix_path, "vkd3d")?;
        Ok(())
    }
}
