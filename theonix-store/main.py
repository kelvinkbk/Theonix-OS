#!/usr/bin/env python3
"""Theonix Store — unified search for native, Flatpak, and UACL apps."""

import os
import sqlite3
import subprocess
import sys

from PyQt6.QtCore import Qt, QThread, pyqtSignal
from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import (
    QApplication,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QMainWindow,
    QMessageBox,
    QPushButton,
    QTabWidget,
    QVBoxLayout,
    QWidget,
)

UACL_DB = os.path.expanduser("~/.config/theonix/uacl.db")


class SearchWorker(QThread):
    results_ready = pyqtSignal(str, list)

    def __init__(self, query: str, category: str):
        super().__init__()
        self.query = query.strip()
        self.category = category

    def run(self):
        try:
            if self.category == "official":
                items = self.search_pacman()
            elif self.category == "flatpak":
                items = self.search_flatpak()
            else:
                items = self.search_uacl()
            self.results_ready.emit(self.category, items)
        except Exception as exc:
            self.results_ready.emit(self.category, [f"Error: {exc}"])

    def search_pacman(self) -> list[str]:
        if not self.query:
            return ["Type to search official packages (pacman)..."]
        result = subprocess.run(
            ["pacman", "-Ss", self.query],
            capture_output=True,
            text=True,
            timeout=30,
        )
        lines = [ln for ln in result.stdout.splitlines() if ln.strip()]
        return lines[:80] or ["No matches in official repositories."]

    def search_flatpak(self) -> list[str]:
        if not self.query:
            return ["Type to search Flatpak apps..."]
        result = subprocess.run(
            ["flatpak", "search", self.query, "--columns=application,name,description"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        lines = [ln for ln in result.stdout.splitlines() if ln.strip()]
        return lines[:80] or ["No Flatpak matches. Install flatpak and add flathub first."]

    def search_uacl(self) -> list[str]:
        if not os.path.isfile(UACL_DB):
            return ["No UACL apps installed yet. Double-click a .exe/.deb/.AppImage to add one."]
        conn = sqlite3.connect(UACL_DB)
        conn.row_factory = sqlite3.Row
        cur = conn.cursor()
        if self.query:
            cur.execute(
                "SELECT name, format_type, launch_count FROM applications "
                "WHERE name LIKE ? OR format_type LIKE ? ORDER BY launch_count DESC",
                (f"%{self.query}%", f"%{self.query}%"),
            )
        else:
            cur.execute(
                "SELECT name, format_type, launch_count FROM applications "
                "ORDER BY launch_count DESC"
            )
        rows = cur.fetchall()
        conn.close()
        if not rows:
            return ["No UACL apps match your search."]
        return [
            f"{row['name']}  [{row['format_type']}]  launches: {row['launch_count']}"
            for row in rows
        ]


class StoreWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Theonix Store")
        self.setMinimumSize(900, 620)
        self.worker = None
        self._build_ui()

    def _build_ui(self):
        root = QWidget()
        self.setCentralWidget(root)
        layout = QVBoxLayout(root)

        header = QLabel("Theonix Store")
        header.setFont(QFont("Inter", 22, QFont.Weight.Bold))
        subtitle = QLabel("Search official packages, Flatpak apps, and Windows/UACL software")
        subtitle.setStyleSheet("color: #888;")
        layout.addWidget(header)
        layout.addWidget(subtitle)

        search_row = QHBoxLayout()
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search apps...")
        self.search_input.returnPressed.connect(self.run_search)
        self.search_btn = QPushButton("Search")
        self.search_btn.clicked.connect(self.run_search)
        search_row.addWidget(self.search_input)
        search_row.addWidget(self.search_btn)
        layout.addLayout(search_row)

        self.tabs = QTabWidget()
        self.lists = {}
        for key, label in (
            ("official", "Official (pacman)"),
            ("flatpak", "Flatpak"),
            ("uacl", "Windows / UACL"),
        ):
            lst = QListWidget()
            self.lists[key] = lst
            self.tabs.addTab(lst, label)
        layout.addWidget(self.tabs)
        self.tabs.currentChanged.connect(lambda _: self.run_search())

        actions = QHBoxLayout()
        self.install_btn = QPushButton("Install / Open Selected")
        self.install_btn.clicked.connect(self.install_selected)
        self.open_manager_btn = QPushButton("Open App Manager")
        self.open_manager_btn.clicked.connect(self.open_app_manager)
        actions.addWidget(self.install_btn)
        actions.addWidget(self.open_manager_btn)
        actions.addStretch()
        layout.addLayout(actions)

        self.run_search()

    def current_category(self) -> str:
        return ("official", "flatpak", "uacl")[self.tabs.currentIndex()]

    def run_search(self):
        query = self.search_input.text()
        category = self.current_category()
        if self.worker and self.worker.isRunning():
            return
        self.worker = SearchWorker(query, category)
        self.worker.results_ready.connect(self.on_results)
        self.worker.start()

    def on_results(self, category: str, items: list[str]):
        lst = self.lists[category]
        lst.clear()
        for item in items:
            lst.addItem(QListWidgetItem(item))

    def install_selected(self):
        category = self.current_category()
        lst = self.lists[category]
        item = lst.currentItem()
        if not item:
            QMessageBox.information(self, "Theonix Store", "Select an item first.")
            return

        text = item.text()
        if category == "official" and text.startswith("    "):
            pkg = text.split()[0]
            reply = QMessageBox.question(
                self,
                "Install package",
                f"Install '{pkg}' with pacman?",
                QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            )
            if reply == QMessageBox.StandardButton.Yes:
                subprocess.Popen(["konsole", "-e", "sudo", "pacman", "-S", "--needed", pkg])
            return

        if category == "flatpak":
            app_id = text.split("\t")[0] if "\t" in text else text.split()[0]
            subprocess.Popen(["konsole", "-e", "flatpak", "install", "-y", "flathub", app_id])
            return

        if category == "uacl":
            name = text.split("  [")[0].strip()
            conn = sqlite3.connect(UACL_DB)
            cur = conn.cursor()
            cur.execute("SELECT id FROM applications WHERE name = ? LIMIT 1", (name,))
            row = cur.fetchone()
            conn.close()
            if row:
                subprocess.Popen(["theonix-uacl", "launch", "--id", row[0]])
            else:
                QMessageBox.information(
                    self,
                    "Theonix Store",
                    "Open the file in Dolphin or use App Manager to launch UACL apps.",
                )

    def open_app_manager(self):
        subprocess.Popen(["theonix-app-manager"])


def main():
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    window = StoreWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
