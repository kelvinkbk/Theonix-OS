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
    // Phase 6.1: intelligence fields
    pub compatibility_rating: i32,  // 1-5 stars, 0 = unrated
    pub launch_count: i32,
    pub last_launch: Option<String>,
    pub known_issues: Option<String>,       // JSON array string
    pub runtime_profile: Option<String>,    // 'gaming', 'office', 'legacy', 'portable'
    pub recommended_runtime: Option<String>,// 'wine', 'proton', 'wine-staging'
    pub gpu_backend: Option<String>,        // 'dxvk', 'vkd3d', 'none'
    pub sandbox_enabled: bool,
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { conn };
        db.init_schema()?;
        db.migrate()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS applications (
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
                compatibility_rating INTEGER DEFAULT 0,
                launch_count INTEGER DEFAULT 0,
                last_launch DATETIME,
                known_issues TEXT,
                runtime_profile TEXT,
                recommended_runtime TEXT,
                gpu_backend TEXT DEFAULT 'none',
                sandbox_enabled BOOLEAN DEFAULT 1,
                installed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS dependencies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_id TEXT,
                dependency_name TEXT,
                installed BOOLEAN DEFAULT 0,
                FOREIGN KEY(app_id) REFERENCES applications(id) ON DELETE CASCADE
            );
        ")?;
        Ok(())
    }

    /// Migrate existing databases by adding new columns if they don't exist
    fn migrate(&self) -> Result<()> {
        let new_columns = vec![
            ("compatibility_rating", "INTEGER DEFAULT 0"),
            ("launch_count", "INTEGER DEFAULT 0"),
            ("last_launch", "DATETIME"),
            ("known_issues", "TEXT"),
            ("runtime_profile", "TEXT"),
            ("recommended_runtime", "TEXT"),
            ("gpu_backend", "TEXT DEFAULT 'none'"),
            ("sandbox_enabled", "BOOLEAN DEFAULT 1"),
        ];

        for (col, def) in new_columns {
            let sql = format!("ALTER TABLE applications ADD COLUMN {} {}", col, def);
            // Ignore error if column already exists
            let _ = self.conn.execute(&sql, []);
        }
        Ok(())
    }

    pub fn upsert_application(&self, app: &Application) -> Result<()> {
        self.conn.execute(
            "INSERT INTO applications (
                id, name, original_file_path, install_path, format_type,
                prefix_path, runtime_version, uses_dxvk, uses_vkd3d,
                desktop_shortcut_path, icon_path,
                compatibility_rating, launch_count, runtime_profile,
                recommended_runtime, gpu_backend, sandbox_enabled
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
            ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                original_file_path=excluded.original_file_path,
                install_path=excluded.install_path,
                format_type=excluded.format_type,
                prefix_path=excluded.prefix_path,
                runtime_version=excluded.runtime_version,
                uses_dxvk=excluded.uses_dxvk,
                uses_vkd3d=excluded.uses_vkd3d,
                runtime_profile=excluded.runtime_profile,
                recommended_runtime=excluded.recommended_runtime,
                gpu_backend=excluded.gpu_backend",
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
                app.compatibility_rating,
                app.launch_count,
                app.runtime_profile,
                app.recommended_runtime,
                app.gpu_backend,
                app.sandbox_enabled,
            ],
        )?;
        Ok(())
    }

    pub fn get_application(&self, id: &str) -> Result<Option<Application>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, original_file_path, install_path, format_type,
                    prefix_path, runtime_version, uses_dxvk, uses_vkd3d,
                    desktop_shortcut_path, icon_path,
                    COALESCE(compatibility_rating, 0),
                    COALESCE(launch_count, 0),
                    last_launch, known_issues, runtime_profile,
                    recommended_runtime,
                    COALESCE(gpu_backend, 'none'),
                    COALESCE(sandbox_enabled, 1)
             FROM applications WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Application {
                id: row.get(0)?,
                name: row.get(1)?,
                original_file_path: row.get(2)?,
                install_path: row.get(3)?,
                format_type: row.get(4)?,
                prefix_path: row.get(5)?,
                runtime_version: row.get(6)?,
                uses_dxvk: row.get(7)?,
                uses_vkd3d: row.get(8)?,
                desktop_shortcut_path: row.get(9)?,
                icon_path: row.get(10)?,
                compatibility_rating: row.get(11)?,
                launch_count: row.get(12)?,
                last_launch: row.get(13)?,
                known_issues: row.get(14)?,
                runtime_profile: row.get(15)?,
                recommended_runtime: row.get(16)?,
                gpu_backend: row.get(17)?,
                sandbox_enabled: row.get(18)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_application(&self, app: &Application) -> Result<()> {
        self.upsert_application(app)
    }

    /// Increment launch count and update last_launch timestamp
    pub fn record_launch(&self, app_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE applications SET
                launch_count = launch_count + 1,
                last_launch = datetime('now')
             WHERE id = ?1",
            params![app_id],
        )?;
        Ok(())
    }

    pub fn update_rating(&self, app_id: &str, rating: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE applications SET compatibility_rating = ?1 WHERE id = ?2",
            params![rating, app_id],
        )?;
        Ok(())
    }

    pub fn get_applications(&self) -> Result<Vec<Application>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, original_file_path, install_path, format_type,
                    prefix_path, runtime_version, uses_dxvk, uses_vkd3d,
                    desktop_shortcut_path, icon_path,
                    COALESCE(compatibility_rating, 0),
                    COALESCE(launch_count, 0),
                    last_launch, known_issues, runtime_profile,
                    recommended_runtime,
                    COALESCE(gpu_backend, 'none'),
                    COALESCE(sandbox_enabled, 1)
             FROM applications"
        )?;

        let app_iter = stmt.query_map([], |row| {
            Ok(Application {
                id: row.get(0)?,
                name: row.get(1)?,
                original_file_path: row.get(2)?,
                install_path: row.get(3)?,
                format_type: row.get(4)?,
                prefix_path: row.get(5)?,
                runtime_version: row.get(6)?,
                uses_dxvk: row.get(7)?,
                uses_vkd3d: row.get(8)?,
                desktop_shortcut_path: row.get(9)?,
                icon_path: row.get(10)?,
                compatibility_rating: row.get(11)?,
                launch_count: row.get(12)?,
                last_launch: row.get(13)?,
                known_issues: row.get(14)?,
                runtime_profile: row.get(15)?,
                recommended_runtime: row.get(16)?,
                gpu_backend: row.get(17)?,
                sandbox_enabled: row.get(18)?,
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
