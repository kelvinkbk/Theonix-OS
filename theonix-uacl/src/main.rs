mod database;
mod detector;
mod runtime;
mod converter;

use clap::{Parser, Subcommand};
use database::{Database, Application};
use detector::{SmartDetector, FileFormat};
use runtime::{RuntimeManager, RuntimeProfile};
use converter::PackageConverter;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about = "Theonix Universal Application Compatibility Layer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch or install an application from a file
    Run {
        #[arg(short, long)]
        file: String,
        /// Override the runtime profile (gaming, office, legacy, portable, development)
        #[arg(short, long, default_value = "auto")]
        profile: String,
    },
    /// Launch a previously registered application by ID
    Launch {
        #[arg(short, long)]
        id: String,
    },
    /// List installed applications
    List,
    /// Uninstall an application by ID
    Uninstall {
        #[arg(short, long)]
        id: String,
    },
}

fn app_id_from_path(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
        .replace(' ', "_")
}

fn format_label(format: &FileFormat) -> &'static str {
    match format {
        FileFormat::WindowsPE => "WindowsPE",
        FileFormat::AppImage => "AppImage",
        FileFormat::ELF => "ELF",
        FileFormat::DebianPackage => "DebianPackage",
        FileFormat::FlatpakBundle => "FlatpakBundle",
        FileFormat::SnapPackage => "SnapPackage",
        FileFormat::ZipArchive => "ZipArchive",
        FileFormat::TarArchive => "TarArchive",
        FileFormat::RpmPackage => "RpmPackage",
        FileFormat::Unknown => "Unknown",
    }
}

fn register_app(
    db: &Database,
    path: &Path,
    format: &FileFormat,
    install_path: &str,
    prefix_path: Option<&str>,
    runtime_version: Option<&str>,
    uses_dxvk: bool,
    runtime_profile: Option<&str>,
) -> anyhow::Result<String> {
    let app_id = app_id_from_path(path);
    let app_name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let app = Application {
        id: app_id.clone(),
        name: app_name,
        original_file_path: path.to_string_lossy().to_string(),
        install_path: install_path.to_string(),
        format_type: format_label(format).to_string(),
        prefix_path: prefix_path.map(|s| s.to_string()),
        runtime_version: runtime_version.map(|s| s.to_string()),
        uses_dxvk,
        uses_vkd3d: false,
        desktop_shortcut_path: None,
        icon_path: None,
        compatibility_rating: 0,
        launch_count: 0,
        last_launch: None,
        known_issues: None,
        runtime_profile: runtime_profile.map(|s| s.to_string()),
        recommended_runtime: runtime_version.map(|s| s.to_string()),
        gpu_backend: if uses_dxvk {
            Some("dxvk".to_string())
        } else {
            Some("none".to_string())
        },
        sandbox_enabled: true,
    };

    db.upsert_application(&app)?;
    println!("Registered '{}' in Theonix App Manager.", app_id);
    Ok(app_id)
}

fn open_db() -> anyhow::Result<Database> {
    let db_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("theonix")
        .join("uacl.db");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(Database::new(db_path)?)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let db = open_db()?;

    match &cli.command {
        Commands::Run { file, profile } => {
            let path = PathBuf::from(file);
            if !path.exists() {
                anyhow::bail!("File does not exist: {}", file);
            }

            let format = SmartDetector::detect_format(&path)?;
            println!("Detected format: {:?}", format);

            match format {
                FileFormat::WindowsPE => {
                    println!("Windows executable detected. Routing to Runtime Engine...");
                    let rm = RuntimeManager::new()?;

                    println!("Scanning for required dependencies...");
                    let deps = SmartDetector::detect_pe_dependencies(&path)?;

                    let rt_profile = RuntimeProfile::from_str(profile);
                    println!("Using runtime profile: {}", rt_profile.name());

                    let app_id = app_id_from_path(&path);
                    let prefix_path = rm.create_wine_prefix(&app_id)?;

                    if !deps.is_empty() {
                        println!("Auto-installing {} dependencies: {:?}", deps.len(), deps);
                        rm.auto_install_dependencies(&prefix_path, &deps)?;
                    }

                    rm.apply_profile(&prefix_path, &rt_profile)?;

                    if rt_profile.uses_dxvk() {
                        rm.install_dxvk(&prefix_path)?;
                    }

                    register_app(
                        &db,
                        &path,
                        &format,
                        &prefix_path.to_string_lossy(),
                        Some(&prefix_path.to_string_lossy()),
                        Some("wine"),
                        rt_profile.uses_dxvk(),
                        Some(rt_profile.name()),
                    )?;

                    rm.spawn_executable(&prefix_path, &path, &[])?;
                    db.record_launch(&app_id)?;
                    println!("Launched '{}' in background.", app_id);
                }

                FileFormat::AppImage | FileFormat::ELF => {
                    let install = path.to_string_lossy().to_string();
                    let app_id = register_app(
                        &db,
                        &path,
                        &format,
                        &install,
                        None,
                        Some("native"),
                        false,
                        Some("portable"),
                    )?;
                    PackageConverter::launch_appimage(&path)?;
                    db.record_launch(&app_id)?;
                }

                FileFormat::DebianPackage => {
                    let install = path.to_string_lossy().to_string();
                    let app_id = register_app(
                        &db,
                        &path,
                        &format,
                        &install,
                        None,
                        Some("native"),
                        false,
                        Some("office"),
                    )?;
                    PackageConverter::install_deb(&path)?;
                    db.record_launch(&app_id)?;
                }

                FileFormat::FlatpakBundle => {
                    let install = path.to_string_lossy().to_string();
                    let app_id = register_app(
                        &db,
                        &path,
                        &format,
                        &install,
                        None,
                        Some("flatpak"),
                        false,
                        Some("portable"),
                    )?;
                    PackageConverter::install_flatpak(&path)?;
                    db.record_launch(&app_id)?;
                }

                FileFormat::SnapPackage => {
                    let install = path.to_string_lossy().to_string();
                    let app_id = register_app(
                        &db,
                        &path,
                        &format,
                        &install,
                        None,
                        Some("snap"),
                        false,
                        Some("portable"),
                    )?;
                    PackageConverter::install_snap(&path)?;
                    db.record_launch(&app_id)?;
                }

                FileFormat::ZipArchive | FileFormat::TarArchive => {
                    let install = path.to_string_lossy().to_string();
                    register_app(
                        &db,
                        &path,
                        &format,
                        &install,
                        None,
                        Some("native"),
                        false,
                        Some("portable"),
                    )?;
                    PackageConverter::handle_archive(&path)?;
                }

                FileFormat::RpmPackage => {
                    println!("RPM package detected. Routing to debtap pipeline...");
                    println!("Note: RPM support coming in Phase 6.5 update.");
                }

                FileFormat::Unknown => {
                    anyhow::bail!("Unknown format. Cannot process this file.");
                }
            }
        }

        Commands::Launch { id } => {
            let app = db
                .get_application(id)?
                .ok_or_else(|| anyhow::anyhow!("Application '{}' not found", id))?;

            match app.format_type.as_str() {
                "WindowsPE" => {
                    let rm = RuntimeManager::new()?;
                    let prefix = app
                        .prefix_path
                        .as_ref()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from(&app.install_path));
                    let exe = PathBuf::from(&app.original_file_path);
                    rm.spawn_executable(&prefix, &exe, &[])?;
                }
                "AppImage" | "ELF" => {
                    PackageConverter::launch_appimage(Path::new(&app.original_file_path))?;
                }
                _ => {
                    anyhow::bail!(
                        "Launch not supported for format '{}'. Re-open the original file.",
                        app.format_type
                    );
                }
            }
            db.record_launch(id)?;
            println!("Launched '{}'.", id);
        }

        Commands::List => {
            let apps = db.get_applications()?;
            if apps.is_empty() {
                println!("No applications installed yet.");
            } else {
                println!(
                    "{:<20} {:<12} {:<10} {:<8} {}",
                    "NAME", "FORMAT", "PROFILE", "LAUNCHES", "RATING"
                );
                println!("{}", "-".repeat(70));
                for app in apps {
                    println!(
                        "{:<20} {:<12} {:<10} {:<8} {}★",
                        app.name,
                        app.format_type,
                        app.runtime_profile.unwrap_or_else(|| "auto".to_string()),
                        app.launch_count,
                        app.compatibility_rating,
                    );
                }
            }
        }

        Commands::Uninstall { id } => {
            if let Some(app) = db.get_application(id)? {
                if let Some(prefix) = &app.prefix_path {
                    let _ = std::fs::remove_dir_all(prefix);
                }
            }
            db.delete_application(id)?;
            println!("Removed '{}' from Theonix App Manager.", id);
        }
    }

    Ok(())
}
