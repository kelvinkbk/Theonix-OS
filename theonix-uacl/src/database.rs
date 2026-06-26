use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

#[derive(Debug)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub original_file_path: String,
    pub install_path: String,
    pub format_type: String,
    pub prefix_path: Option<String>,
    pub runtime_version: Option<String>,
    pub uses_dxvk: bool,
    pub uses_vkd3d: bool,
    pub desktop_shortcut_path: Option<String>,
    pub icon_path: Option<String>,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS applications (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                original_file_path TEXT,
                install_path TEXT,
                format_type TEXT,
                prefix_path TEXT,
                runtime_version TEXT,
                uses_dxvk BOOLEAN DEFAULT 0,
                uses_vkd3d BOOLEAN DEFAULT 0,
                desktop_shortcut_path TEXT,
                icon_path TEXT,
                installed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_id TEXT,
                dependency_name TEXT,
                FOREIGN KEY(app_id) REFERENCES applications(id) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(())
    }

    pub fn insert_application(&self, app: &Application) -> Result<()> {
        self.conn.execute(
            "INSERT INTO applications (
                id, name, original_file_path, install_path, format_type,
                prefix_path, runtime_version, uses_dxvk, uses_vkd3d,
                desktop_shortcut_path, icon_path
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                app.id,
                app.name,
                app.original_file_path,
                app.install_path,
                app.format_type,
                app.prefix_path,
                app.runtime_version,
                app.uses_dxvk,
                app.uses_vkd3d,
                app.desktop_shortcut_path,
                app.icon_path,
            ],
        )?;
        Ok(())
    }

    pub fn get_applications(&self) -> Result<Vec<Application>> {
        let mut stmt = self.conn.prepare("SELECT * FROM applications")?;
        let app_iter = stmt.query_map([], |row| {
            Ok(Application {
                id: row.get("id")?,
                name: row.get("name")?,
                original_file_path: row.get("original_file_path")?,
                install_path: row.get("install_path")?,
                format_type: row.get("format_type")?,
                prefix_path: row.get("prefix_path")?,
                runtime_version: row.get("runtime_version")?,
                uses_dxvk: row.get("uses_dxvk")?,
                uses_vkd3d: row.get("uses_vkd3d")?,
                desktop_shortcut_path: row.get("desktop_shortcut_path")?,
                icon_path: row.get("icon_path")?,
            })
        })?;

        let mut apps = Vec::new();
        for app in app_iter {
            apps.push(app?);
        }
        Ok(apps)
    }

    pub fn delete_application(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM applications WHERE id = ?1", params![id])?;
        Ok(())
    }
}
