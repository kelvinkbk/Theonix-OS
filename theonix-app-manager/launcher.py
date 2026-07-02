import sys
import subprocess
import os
import threading
from PyQt6.QtWidgets import (
    QApplication, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QProgressBar, QPushButton
)
from PyQt6.QtCore import Qt, QTimer, pyqtSignal, QObject, QThread
from PyQt6.QtGui import QFont, QPixmap, QColor, QPainter, QPainterPath

STEPS_EXE = [
    ("🔍", "Detecting application type..."),
    ("📋", "Scanning for required dependencies..."),
    ("⚙️", "Creating isolated environment..."),
    ("📦", "Installing missing dependencies..."),
    ("🚀", "Launching application..."),
]

STEPS_DEB = [
    ("🔍", "Detecting package format..."),
    ("🔄", "Converting to native package..."),
    ("📦", "Installing via pacman..."),
    ("✅", "Installation complete!"),
]

STEPS_APPIMAGE = [
    ("🔍", "Detecting AppImage format..."),
    ("🔒", "Marking executable..."),
    ("🚀", "Launching application..."),
]

STEPS_FLATPAK = [
    ("🔍", "Detecting Flatpak bundle..."),
    ("📦", "Installing Flatpak package..."),
    ("✅", "Installation complete!"),
]


class WorkerSignals(QObject):
    step_done = pyqtSignal(int)
    finished = pyqtSignal(bool, str)


class LaunchWorker(QThread):
    def __init__(self, file_path, format_type):
        super().__init__()
        self.file_path = file_path
        self.format_type = format_type
        self.signals = WorkerSignals()

    def run(self):
        try:
            if self.format_type == "exe":
                self.signals.step_done.emit(0)
                self.msleep(600)

                # Get app id from filename
                app_id = os.path.splitext(os.path.basename(self.file_path))[0].lower().replace(" ", "_")
                self.signals.step_done.emit(1)
                self.msleep(400)

                self.signals.step_done.emit(2)
                
                # Run theonix-uacl in background
                result = subprocess.run(
                    ["theonix-uacl", "run", "--file", self.file_path],
                    capture_output=True, text=True
                )
                if result.returncode != 0:
                    err = (result.stderr or result.stdout or "UACL failed").strip()
                    self.signals.finished.emit(False, err)
                    return

                self.signals.step_done.emit(3)
                self.msleep(200)
                self.signals.finished.emit(True, "")

            elif self.format_type == "deb":
                self.signals.step_done.emit(0)
                self.msleep(400)
                self.signals.step_done.emit(1)

                result = subprocess.run(
                    ["theonix-uacl", "run", "--file", self.file_path],
                    capture_output=True, text=True
                )
                if result.returncode != 0:
                    err = (result.stderr or result.stdout or "UACL failed").strip()
                    self.signals.finished.emit(False, err)
                    return

                self.signals.step_done.emit(2)
                self.msleep(300)
                self.signals.step_done.emit(3)
                self.signals.finished.emit(True, "")

            elif self.format_type == "appimage":
                self.signals.step_done.emit(0)
                self.msleep(400)
                self.signals.step_done.emit(1)
                self.msleep(200)

                result = subprocess.run(
                    ["theonix-uacl", "run", "--file", self.file_path],
                    capture_output=True, text=True
                )
                if result.returncode != 0:
                    err = (result.stderr or result.stdout or "UACL failed").strip()
                    self.signals.finished.emit(False, err)
                    return

                self.signals.step_done.emit(2)
                self.signals.finished.emit(True, "")

        except Exception as e:
            self.signals.finished.emit(False, str(e))


class TheonixLauncher(QWidget):
    def __init__(self, file_path):
        super().__init__()
        self.file_path = file_path
        self.app_name = os.path.splitext(os.path.basename(file_path))[0]
        self.ext = os.path.splitext(file_path)[1].lower().lstrip(".")
        
        if self.ext in ("exe", "msi"):
            self.format_type = "exe"
            self.steps = STEPS_EXE
        elif self.ext == "deb":
            self.format_type = "deb"
            self.steps = STEPS_DEB
        elif self.ext == "appimage":
            self.format_type = "appimage"
            self.steps = STEPS_APPIMAGE
        else:
            self.format_type = "exe"
            self.steps = STEPS_EXE

        self.current_step = 0
        self.step_labels = []
        self.init_ui()

    def init_ui(self):
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.WindowStaysOnTopHint)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setFixedSize(440, 300)

        # Center on screen
        screen = QApplication.primaryScreen().geometry()
        self.move(
            screen.center().x() - self.width() // 2,
            screen.center().y() - self.height() // 2
        )

        outer_layout = QVBoxLayout(self)
        outer_layout.setContentsMargins(0, 0, 0, 0)

        # Main card container
        self.card = QWidget()
        self.card.setObjectName("card")
        self.card.setStyleSheet("""
            QWidget#card {
                background-color: #1a1a2e;
                border-radius: 16px;
                border: 1px solid #30304a;
            }
        """)

        card_layout = QVBoxLayout(self.card)
        card_layout.setContentsMargins(32, 28, 32, 28)
        card_layout.setSpacing(16)

        # Header: Logo + title
        header = QHBoxLayout()
        logo_label = QLabel("🖥")
        logo_label.setStyleSheet("font-size: 36px;")
        
        title_col = QVBoxLayout()
        title = QLabel(f"Opening {self.app_name}")
        title.setStyleSheet("color: #ffffff; font-size: 17px; font-weight: bold;")
        title.setFont(QFont("Inter", 13, QFont.Weight.Bold))

        subtitle = QLabel("Theonix is setting up your application")
        subtitle.setStyleSheet("color: #8888aa; font-size: 12px;")

        title_col.addWidget(title)
        title_col.addWidget(subtitle)

        header.addWidget(logo_label)
        header.addSpacing(12)
        header.addLayout(title_col)
        header.addStretch()
        card_layout.addLayout(header)

        # Divider
        divider = QWidget()
        divider.setFixedHeight(1)
        divider.setStyleSheet("background-color: #2d2d4a;")
        card_layout.addWidget(divider)

        # Steps list
        self.steps_container = QVBoxLayout()
        self.steps_container.setSpacing(10)

        for i, (icon, text) in enumerate(self.steps):
            row = QHBoxLayout()
            
            # Status icon (checkmark once done, spinner icon while active)
            status_lbl = QLabel("○")
            status_lbl.setFixedWidth(22)
            status_lbl.setStyleSheet("color: #444466; font-size: 14px;")
            
            text_lbl = QLabel(f"{icon}  {text}")
            text_lbl.setStyleSheet("color: #555577; font-size: 12px;")

            row.addWidget(status_lbl)
            row.addWidget(text_lbl)
            row.addStretch()

            self.step_labels.append((status_lbl, text_lbl))
            self.steps_container.addLayout(row)

        card_layout.addLayout(self.steps_container)

        # Progress bar
        self.progress = QProgressBar()
        self.progress.setRange(0, len(self.steps))
        self.progress.setValue(0)
        self.progress.setTextVisible(False)
        self.progress.setFixedHeight(6)
        self.progress.setStyleSheet("""
            QProgressBar {
                background-color: #2d2d4a;
                border-radius: 3px;
            }
            QProgressBar::chunk {
                background: qlineargradient(x1:0, y1:0, x2:1, y2:0,
                    stop:0 #7c3aed, stop:1 #a855f7);
                border-radius: 3px;
            }
        """)
        card_layout.addWidget(self.progress)

        # Cancel button
        self.cancel_btn = QPushButton("Cancel")
        self.cancel_btn.setStyleSheet("""
            QPushButton {
                background: transparent;
                color: #666688;
                border: 1px solid #333355;
                border-radius: 6px;
                padding: 6px 16px;
                font-size: 12px;
            }
            QPushButton:hover {
                background: #2d2d4a;
                color: #aaaacc;
            }
        """)
        self.cancel_btn.clicked.connect(self.close)

        btn_row = QHBoxLayout()
        btn_row.addStretch()
        btn_row.addWidget(self.cancel_btn)
        card_layout.addLayout(btn_row)

        outer_layout.addWidget(self.card)

        # Kick off the worker
        self.worker = LaunchWorker(self.file_path, self.format_type)
        self.worker.signals.step_done.connect(self.on_step_done)
        self.worker.signals.finished.connect(self.on_finished)
        self.worker.start()

    def on_step_done(self, step_index):
        if step_index < len(self.step_labels):
            # Mark previous as done
            if step_index > 0:
                prev_status, prev_text = self.step_labels[step_index - 1]
                prev_status.setText("✔")
                prev_status.setStyleSheet("color: #a855f7; font-size: 14px; font-weight: bold;")
                prev_text.setStyleSheet("color: #aaaacc; font-size: 12px;")

            # Highlight current
            cur_status, cur_text = self.step_labels[step_index]
            cur_status.setText("▶")
            cur_status.setStyleSheet("color: #7c3aed; font-size: 12px;")
            cur_text.setStyleSheet("color: #ffffff; font-size: 12px; font-weight: bold;")

            self.progress.setValue(step_index + 1)

    def on_finished(self, success, error):
        if not success:
            if self.step_labels:
                last_status, last_text = self.step_labels[-1]
                last_status.setText("✖")
                last_status.setStyleSheet("color: #ff5555; font-size: 14px; font-weight: bold;")
                last_text.setStyleSheet("color: #ff8888; font-size: 12px;")
            self.cancel_btn.setText("Close")
            if error:
                self.cancel_btn.setToolTip(error)
            return

        # Mark last step done
        if self.step_labels:
            last_status, last_text = self.step_labels[-1]
            last_status.setText("✔")
            last_status.setStyleSheet("color: #a855f7; font-size: 14px; font-weight: bold;")
            last_text.setStyleSheet("color: #aaaacc; font-size: 12px;")
        self.progress.setValue(len(self.steps))

        # Close after a brief moment
        QTimer.singleShot(1200, self.close)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: theonix-launcher <file>")
        sys.exit(1)

    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    
    window = TheonixLauncher(sys.argv[1])
    window.show()
    sys.exit(app.exec())
