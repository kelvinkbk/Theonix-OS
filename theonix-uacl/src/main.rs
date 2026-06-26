mod database;
mod detector;
mod runtime;
mod converter;

use clap::{Parser, Subcommand};
use database::Database;
use detector::{SmartDetector, FileFormat};
use runtime::RuntimeManager;
use converter::PackageConverter;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
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
    },
    /// List installed applications
    List,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let db_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("theonix")
        .join("uacl.db");
    
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::new(db_path)?;

    match &cli.command {
        Commands::Run { file } => {
            let path = PathBuf::from(file);
            if !path.exists() {
                anyhow::bail!("File does not exist: {}", file);
            }

            let format = SmartDetector::detect_format(&path)?;
            println!("Detected format: {:?}", format);

            match format {
                FileFormat::WindowsPE => {
                    println!("Windows executable detected. Intercepting and routing to Runtime Manager...");
                    let rm = RuntimeManager::new()?;
                    
                    // Use a hash of the file path or name as a unique app ID for now
                    let app_id = path.file_stem().unwrap().to_string_lossy().to_string();
                    
                    let prefix_path = rm.create_wine_prefix(&app_id)?;
                    
                    // Simple heuristic: if it contains "setup" or "install", we should probably install it.
                    // For now, just run it.
                    rm.run_executable(&prefix_path, &path, &[])?;
                }
                FileFormat::AppImage => {
                    println!("AppImage detected. Integrating into desktop...");
                    PackageConverter::launch_appimage(&path)?;
                }
                FileFormat::DebianPackage => {
                    println!("Debian package detected. Routing to debtap conversion pipeline...");
                    PackageConverter::install_deb(&path)?;
                }
                FileFormat::RpmPackage => {
                    println!("RPM package detected. Routing to debtap/alien pipeline...");
                    println!("Error: RPM conversion not fully implemented yet. Please use debtap manually.");
                }
                FileFormat::ELF => {
                    println!("Native ELF binary detected. Marking as executable and running...");
                    PackageConverter::launch_appimage(&path)?; // Works for ELF too
                }
                FileFormat::Unknown => {
                    println!("Unknown format. Attempting standard execution...");
                }
            }
        }
        Commands::List => {
            let apps = db.get_applications()?;
            for app in apps {
                println!("{:?}", app);
            }
        }
    }

    Ok(())
}
