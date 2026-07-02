import sys
import sqlite3
import os
import subprocess
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QListWidget, QListWidgetItem, QLabel, QPushButton, QMessageBox,
    QTabWidget, QFormLayout, QCheckBox
)
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QFont

DB_PATH = os.path.expanduser("~/.config/theonix/uacl.db")

SCHEMA_SQL = """
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
"""

class Database:
    def __init__(self):
        os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
        self.conn = sqlite3.connect(DB_PATH)
        self.conn.row_factory = sqlite3.Row
        self.conn.executescript(SCHEMA_SQL)
        self.conn.commit()

    def get_applications(self):
        cursor = self.conn.cursor()
        cursor.execute(
            "SELECT * FROM applications ORDER BY COALESCE(last_launch, installed_at) DESC"
        )
        return cursor.fetchall()

    def delete_application(self, app_id):
        cursor = self.conn.cursor()
        cursor.execute("DELETE FROM applications WHERE id = ?", (app_id,))
        self.conn.commit()

class AppManagerWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Theonix App Manager (UACL)")
        self.setMinimumSize(800, 600)
        self.db = Database()
        self.init_ui()

        self.refresh_timer = QTimer(self)
        self.refresh_timer.timeout.connect(self.refresh_list)
        self.refresh_timer.start(2000)

    def showEvent(self, event):
        super().showEvent(event)
        self.refresh_list()

    def init_ui(self):
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QHBoxLayout(central_widget)

        self.app_list = QListWidget()
        self.app_list.setMinimumWidth(250)
        self.app_list.itemSelectionChanged.connect(self.on_app_selected)
        main_layout.addWidget(self.app_list)

        right_panel = QWidget()
        self.right_layout = QVBoxLayout(right_panel)
        main_layout.addWidget(right_panel, stretch=1)

        self.title_label = QLabel("Select an application")
        self.title_label.setFont(QFont("Arial", 20, QFont.Weight.Bold))
        self.right_layout.addWidget(self.title_label)

        self.tabs = QTabWidget()
        self.right_layout.addWidget(self.tabs)

        self.details_tab = QWidget()
        self.details_layout = QFormLayout(self.details_tab)
        self.tabs.addTab(self.details_tab, "Details")

        self.lbl_format = QLabel("")
        self.lbl_runtime = QLabel("")
        self.lbl_prefix = QLabel("")
        self.lbl_launches = QLabel("")

        self.details_layout.addRow("Format:", self.lbl_format)
        self.details_layout.addRow("Runtime:", self.lbl_runtime)
        self.details_layout.addRow("Prefix Path:", self.lbl_prefix)
        self.details_layout.addRow("Launches:", self.lbl_launches)

        self.settings_tab = QWidget()
        self.settings_layout = QVBoxLayout(self.settings_tab)
        self.tabs.addTab(self.settings_tab, "Compatibility")

        self.chk_dxvk = QCheckBox("Enable DXVK")
        self.chk_vkd3d = QCheckBox("Enable VKD3D")
        self.settings_layout.addWidget(self.chk_dxvk)
        self.settings_layout.addWidget(self.chk_vkd3d)
        self.settings_layout.addStretch()

        button_layout = QHBoxLayout()
        self.btn_refresh = QPushButton("Refresh")
        self.btn_launch = QPushButton("Launch Application")
        self.btn_uninstall = QPushButton("Uninstall")
        self.btn_uninstall.setStyleSheet("background-color: #d9534f; color: white;")

        self.btn_refresh.clicked.connect(self.refresh_list)
        self.btn_launch.clicked.connect(self.launch_app)
        self.btn_uninstall.clicked.connect(self.uninstall_app)

        button_layout.addWidget(self.btn_refresh)
        button_layout.addWidget(self.btn_launch)
        button_layout.addWidget(self.btn_uninstall)
        self.right_layout.addLayout(button_layout)

        self.refresh_list()
        self.update_details_panel(None)

    def refresh_list(self):
        selected_id = None
        items = self.app_list.selectedItems()
        if items:
            selected_id = items[0].data(Qt.ItemDataRole.UserRole).get("id")

        self.app_list.clear()
        apps = self.db.get_applications()
        for app in apps:
            item = QListWidgetItem(app["name"])
            item.setData(Qt.ItemDataRole.UserRole, dict(app))
            self.app_list.addItem(item)
            if selected_id and app["id"] == selected_id:
                item.setSelected(True)

        if not apps:
            self.update_details_panel(None)

    def on_app_selected(self):
        items = self.app_list.selectedItems()
        if not items:
            self.update_details_panel(None)
            return
        app_data = items[0].data(Qt.ItemDataRole.UserRole)
        self.update_details_panel(app_data)

    def update_details_panel(self, app_data):
        if not app_data:
            self.title_label.setText("Select an application")
            self.lbl_format.setText("")
            self.lbl_runtime.setText("")
            self.lbl_prefix.setText("")
            self.lbl_launches.setText("")
            self.btn_launch.setEnabled(False)
            self.btn_uninstall.setEnabled(False)
            return

        self.title_label.setText(app_data["name"])
        self.lbl_format.setText(app_data.get("format_type", "Unknown"))
        self.lbl_runtime.setText(app_data.get("runtime_version", "Native"))
        self.lbl_prefix.setText(app_data.get("prefix_path") or app_data.get("install_path", "N/A"))
        self.lbl_launches.setText(str(app_data.get("launch_count", 0)))

        self.chk_dxvk.setChecked(bool(app_data.get("uses_dxvk", False)))
        self.chk_vkd3d.setChecked(bool(app_data.get("uses_vkd3d", False)))

        self.btn_launch.setEnabled(True)
        self.btn_uninstall.setEnabled(True)

    def launch_app(self):
        items = self.app_list.selectedItems()
        if not items:
            return
        app_data = items[0].data(Qt.ItemDataRole.UserRole)
        result = subprocess.run(
            ["theonix-uacl", "launch", "--id", app_data["id"]],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            QMessageBox.warning(
                self,
                "Launch failed",
                result.stderr or result.stdout or "Unknown error",
            )
        else:
            self.refresh_list()

    def uninstall_app(self):
        items = self.app_list.selectedItems()
        if not items:
            return
        app_data = items[0].data(Qt.ItemDataRole.UserRole)

        reply = QMessageBox.question(
            self,
            "Uninstall",
            f"Uninstall {app_data['name']}?\nThis removes the Wine prefix and registry entry.",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            subprocess.run(
                ["theonix-uacl", "uninstall", "--id", app_data["id"]],
                capture_output=True,
                text=True,
            )
            self.refresh_list()
            QMessageBox.information(self, "Uninstalled", f"{app_data['name']} has been removed.")

if __name__ == "__main__":
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    window = AppManagerWindow()
    window.show()
    sys.exit(app.exec())
