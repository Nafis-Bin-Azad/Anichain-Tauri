import sys
if sys.version_info[0] == 3 and sys.version_info[1] < 8:
    raise ImportError("This application requires Python 3.8 or higher")

from PyQt6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, 
                           QHBoxLayout, QLabel, QTabWidget, QScrollArea, 
                           QPushButton, QLineEdit, QGridLayout, QFrame,
                           QStackedWidget, QListWidget, QFileDialog, QMessageBox,
                           QTextEdit, QDialog, QButtonGroup, QSizePolicy, QProgressBar)
from PyQt6.QtCore import Qt, QThread, pyqtSignal, QSize, QTimer, QPropertyAnimation, QPoint, QEasingCurve
from PyQt6.QtGui import QPixmap, QImage, QPalette, QColor, QFont
import os
from datetime import datetime, timedelta
from anime_backend import AnimeManager

class ImageLoader(QThread):
    image_loaded = pyqtSignal(str, QPixmap)
    
    def __init__(self, title, manager):
        super().__init__()
        self.title = title
        self.manager = manager
        
    def run(self):
        image_path = self.manager.fetch_anime_image(self.title)
        if image_path:
            pixmap = QPixmap(image_path)
            scaled_pixmap = pixmap.scaled(300, 420, Qt.AspectRatioMode.KeepAspectRatio, 
                                        Qt.TransformationMode.SmoothTransformation)
            self.image_loaded.emit(self.title, scaled_pixmap)

class FlippableCard(QFrame):
    def __init__(self):
        super().__init__()
        self.is_flipped = False
        
    def flip_card(self):
        if self.is_flipped:
            self.back_widget.hide()
            self.front_widget.show()
        else:
            self.front_widget.hide()
            self.back_widget.show()
        self.is_flipped = not self.is_flipped

class AnimeInfoLoader(QThread):
    info_loaded = pyqtSignal(str, str)
    
    def __init__(self, title):
        super().__init__()
        self.title = title
        
    def run(self):
        try:
            import requests
            import time
            
            # Clean up title for search
            search_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
            
            # Search for anime
            response = requests.get(
                f"https://api.jikan.moe/v4/anime",
                params={"q": search_title, "limit": 1},
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                if data["data"]:
                    anime = data["data"][0]
                    synopsis = anime.get("synopsis", "No description available.")
                    self.info_loaded.emit(self.title, synopsis)
                    return
                    
            self.info_loaded.emit(self.title, "No description available.")
            
        except Exception as e:
            self.info_loaded.emit(self.title, f"Failed to load description: {str(e)}")
            
class AnimeCard(FlippableCard):
    clicked = pyqtSignal(str)
    
    def __init__(self, title, manager, parent=None):
        super().__init__()
        self.title = title
        self.manager = manager
        self.original_pixmap = None
        self.setup_ui()
        
    def setup_ui(self):
        self.setObjectName("animeCard")
        self.setStyleSheet("""
            #animeCard {
                background-color: white;
                border-radius: 10px;
                border: 1px solid #e0e0e0;
            }
            #animeCard:hover {
                border: 1px solid #007AFF;
            }
            QLabel {
                color: #333333;
            }
        """)
        
        # Main layout
        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(10, 10, 10, 10)
        self.layout.setSpacing(8)
        
        # Create front and back widgets in the same layout
        self.front_widget = QWidget()
        self.back_widget = QWidget()
        self.setup_front()
        self.setup_back()
        
        # Add both to layout
        self.layout.addWidget(self.front_widget)
        self.layout.addWidget(self.back_widget)
        self.back_widget.hide()
        
        # Load image
        clean_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        self.loader = ImageLoader(clean_title, self.manager)
        self.loader.image_loaded.connect(self.set_image)
        self.loader.start()
        
    def setup_front(self):
        layout = QVBoxLayout(self.front_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(8)
        
        # Image
        self.image_label = QLabel()
        self.image_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.image_label.setFixedSize(200, 280)  # Fixed size for stability
        layout.addWidget(self.image_label)
        
        # Title
        clean_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        self.title_label = QLabel(clean_title)
        self.title_label.setWordWrap(True)
        self.title_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.title_label.setStyleSheet("font-weight: bold; font-size: 14px;")
        self.title_label.setFixedHeight(40)  # Fixed height for title
        layout.addWidget(self.title_label)
        
        # Episode info
        episode_info = self.title.split(" - ")[-1].split("[")[0].strip()
        self.episode_label = QLabel(episode_info)
        self.episode_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.episode_label.setStyleSheet("color: #666666;")
        self.episode_label.setFixedHeight(20)  # Fixed height for episode info
        layout.addWidget(self.episode_label)
        
        # Status indicator
        self.status_label = QLabel()
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.status_label.setFixedHeight(20)  # Fixed height for status
        self.update_status()
        layout.addWidget(self.status_label)
        
    def setup_back(self):
        layout = QVBoxLayout(self.back_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(10)
        
        # Info container
        info_frame = QFrame()
        info_frame.setStyleSheet("""
            QFrame {
                background-color: #f5f5f7;
                border-radius: 10px;
                padding: 15px;
            }
        """)
        info_layout = QVBoxLayout(info_frame)
        
        # Title
        clean_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        title_label = QLabel(clean_title)
        title_label.setStyleSheet("font-weight: bold; font-size: 16px;")
        title_label.setWordWrap(True)
        info_layout.addWidget(title_label)
        
        # Episode
        episode_info = self.title.split(" - ")[-1].split("[")[0].strip()
        episode_label = QLabel(f"Episode: {episode_info}")
        episode_label.setStyleSheet("color: #666666;")
        info_layout.addWidget(episode_label)
        
        # Description
        self.desc_label = QLabel("Loading anime description...")
        self.desc_label.setWordWrap(True)
        self.desc_label.setStyleSheet("color: #333333;")
        info_layout.addWidget(self.desc_label)
        
        # Load description
        self.info_loader = AnimeInfoLoader(self.title)
        self.info_loader.info_loaded.connect(self.update_description)
        
        # Track/Untrack button
        clean_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        is_tracked = any(clean_title in anime for anime in self.manager.tracked_anime)
        
        self.track_btn = QPushButton("Untrack Series" if is_tracked else "Track Series")
        self.track_btn.setStyleSheet("""
            QPushButton {
                background-color: """ + ("#ff3b30" if is_tracked else "#007AFF") + """;
                color: white;
                border-radius: 5px;
                padding: 8px;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: """ + ("#ff453a" if is_tracked else "#0066CC") + """;
            }
        """)
        self.track_btn.clicked.connect(lambda: self.clicked.emit(self.title))
        
        layout.addWidget(info_frame)
        layout.addWidget(self.track_btn)
        
    def set_image(self, title, pixmap):
        if title == self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]:
            self.original_pixmap = pixmap
            scaled_pixmap = pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def resizeEvent(self, event):
        super().resizeEvent(event)
        if self.original_pixmap:
            scaled_pixmap = self.original_pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def mousePressEvent(self, event):
        self.flip_card()
        if not self.is_flipped:
            # Start loading description when card is flipped to back
            self.info_loader.start()
        
    def update_status(self):
        clean_title = self.title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        is_tracked = any(clean_title in anime for anime in self.manager.tracked_anime)
        self.status_label.setText("✓ Tracking" if is_tracked else "Click to View Info")
        self.status_label.setStyleSheet(
            "color: #00b894; font-weight: bold;" if is_tracked else "color: #0984e3;"
        )
        if hasattr(self, 'track_btn'):
            self.track_btn.setText("Untrack Series" if is_tracked else "Track Series")
            self.track_btn.setStyleSheet("""
                QPushButton {
                    background-color: """ + ("#ff3b30" if is_tracked else "#007AFF") + """;
                    color: white;
                    border-radius: 5px;
                    padding: 8px;
                    font-size: 14px;
                }
                QPushButton:hover {
                    background-color: """ + ("#ff453a" if is_tracked else "#0066CC") + """;
                }
            """)
        
    def update_description(self, title, description):
        if title == self.title:
            self.desc_label.setText(description)
            
    def flip_card(self):
        super().flip_card()
        if not self.is_flipped:
            # Start loading description when card is flipped to back
            self.info_loader.start()

class QBittorrentDialog(QDialog):
    def __init__(self, manager, parent=None):
        super().__init__(parent)
        self.manager = manager
        self.setWindowTitle("qBittorrent Connection")
        self.setFixedWidth(400)
        
        layout = QVBoxLayout(self)
        
        # Host
        layout.addWidget(QLabel("Host:"))
        self.host_entry = QLineEdit(self.manager.settings["qb_host"])
        layout.addWidget(self.host_entry)
        
        # Username
        layout.addWidget(QLabel("Username:"))
        self.username_entry = QLineEdit(self.manager.settings["qb_username"])
        layout.addWidget(self.username_entry)
        
        # Password
        layout.addWidget(QLabel("Password:"))
        self.password_entry = QLineEdit(self.manager.settings["qb_password"])
        self.password_entry.setEchoMode(QLineEdit.EchoMode.Password)
        layout.addWidget(self.password_entry)
        
        # Buttons
        button_layout = QHBoxLayout()
        
        connect_btn = QPushButton("Connect")
        connect_btn.clicked.connect(self.try_connect)
        connect_btn.setStyleSheet("""
            QPushButton {
                background-color: #007AFF;
                color: white;
                border-radius: 5px;
                padding: 8px 16px;
            }
            QPushButton:hover {
                background-color: #0066CC;
            }
        """)
        
        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.reject)
        cancel_btn.setStyleSheet("""
            QPushButton {
                background-color: #ff3b30;
                color: white;
                border-radius: 5px;
                padding: 8px 16px;
            }
            QPushButton:hover {
                background-color: #ff453a;
            }
        """)
        
        button_layout.addWidget(connect_btn)
        button_layout.addWidget(cancel_btn)
        layout.addLayout(button_layout)
        
    def try_connect(self):
        new_settings = self.manager.settings.copy()
        new_settings.update({
            "qb_host": self.host_entry.text(),
            "qb_username": self.username_entry.text(),
            "qb_password": self.password_entry.text()
        })
        
        self.manager.save_settings(new_settings)
        if self.manager.setup_qbittorrent():
            self.accept()
        else:
            QMessageBox.critical(self, "Error", "Failed to connect to qBittorrent")
            
class TrackedAnimeCard(FlippableCard):
    def __init__(self, series_name, manager, parent=None):
        super().__init__()
        self.series_name = series_name
        self.manager = manager
        self.original_pixmap = None
        self.setup_ui()
        
    def setup_ui(self):
        self.setObjectName("trackedCard")
        self.setStyleSheet("""
            #trackedCard {
                background-color: white;
                border-radius: 10px;
                border: 1px solid #e0e0e0;
            }
            #trackedCard[ended="true"] {
                border: 2px solid #ff3b30;
            }
            #trackedCard:hover {
                border: 1px solid #007AFF;
            }
            #trackedCard[ended="true"]:hover {
                border: 2px solid #ff453a;
            }
            QLabel {
                color: #333333;
            }
        """)
        
        # Main layout
        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(10, 10, 10, 10)
        self.layout.setSpacing(8)
        
        # Create front and back widgets in the same layout
        self.front_widget = QWidget()
        self.back_widget = QWidget()
        self.setup_front()
        self.setup_back()
        
        # Add both to layout
        self.layout.addWidget(self.front_widget)
        self.layout.addWidget(self.back_widget)
        self.back_widget.hide()
        
        # Load image
        self.loader = ImageLoader(self.series_name, self.manager)
        self.loader.image_loaded.connect(self.set_image)
        self.loader.start()
        
    def setup_front(self):
        layout = QVBoxLayout(self.front_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(8)
        
        # Image
        self.image_label = QLabel()
        self.image_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.image_label.setFixedSize(200, 280)  # Fixed size for stability
        layout.addWidget(self.image_label)
        
        # Title
        title_label = QLabel(self.series_name)
        title_label.setWordWrap(True)
        title_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        title_label.setStyleSheet("font-weight: bold; font-size: 14px;")
        layout.addWidget(title_label)
        
        # Status
        self.status_label = QLabel()
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.status_label.setWordWrap(True)
        layout.addWidget(self.status_label)
        
        # End notice (hidden by default)
        self.end_notice = QLabel()
        self.end_notice.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.end_notice.setWordWrap(True)
        self.end_notice.setStyleSheet("""
            QLabel {
                color: #ff3b30;
                font-size: 12px;
                padding: 5px;
                background-color: #fff2f2;
                border-radius: 5px;
            }
        """)
        self.end_notice.hide()
        layout.addWidget(self.end_notice)
        
        self.update_status()
        
    def setup_back(self):
        layout = QVBoxLayout(self.back_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(10)
        
        # Info container
        info_frame = QFrame()
        info_frame.setStyleSheet("""
            QFrame {
                background-color: #f5f5f7;
                border-radius: 10px;
                padding: 15px;
            }
        """)
        info_layout = QVBoxLayout(info_frame)
        
        # Series name
        name_label = QLabel(self.series_name)
        name_label.setStyleSheet("font-weight: bold; font-size: 16px;")
        name_label.setWordWrap(True)
        info_layout.addWidget(name_label)
        
        # Last episode
        last_episode = self.get_last_episode()
        episode_label = QLabel(f"Last episode: {last_episode}")
        episode_label.setStyleSheet("color: #666666;")
        info_layout.addWidget(episode_label)
        
        # Next episode countdown
        self.countdown_label = QLabel()
        self.countdown_label.setStyleSheet("""
            QLabel {
                color: #00b894;
                font-weight: bold;
                padding: 5px;
                background-color: #e6fff7;
                border-radius: 5px;
            }
        """)
        info_layout.addWidget(self.countdown_label)
        self.update_countdown()
        
        # Status
        status = self.check_series_status()
        status_label = QLabel(f"Status: {status}")
        status_label.setStyleSheet(
            "color: #00b894; font-weight: bold;" if status == "Ongoing" else "color: #d63031; font-weight: bold;"
        )
        info_layout.addWidget(status_label)
        
        layout.addWidget(info_frame)
        
        # Untrack button
        untrack_btn = QPushButton("Stop Tracking")
        untrack_btn.setStyleSheet("""
            QPushButton {
                background-color: #ff3b30;
                color: white;
                border-radius: 5px;
                padding: 8px;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #ff453a;
            }
        """)
        untrack_btn.clicked.connect(self.untrack_series)
        layout.addWidget(untrack_btn)
        
        # Start countdown timer
        self.countdown_timer = QTimer()
        self.countdown_timer.timeout.connect(self.update_countdown)
        self.countdown_timer.start(60000)  # Update every minute
        
    def set_image(self, title, pixmap):
        if title == self.series_name:
            self.original_pixmap = pixmap
            scaled_pixmap = pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def resizeEvent(self, event):
        super().resizeEvent(event)
        if self.original_pixmap:
            scaled_pixmap = self.original_pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def mousePressEvent(self, event):
        self.flip_card()
        
    def get_last_episode(self):
        downloaded_files = self.manager.get_downloaded_files()
        latest_episode = "None"
        
        for file in downloaded_files:
            if file.endswith(".mkv") and self.series_name in file:
                try:
                    episode = file.split(" - ")[1].split("[")[0].strip()
                    latest_episode = episode
                except:
                    pass
                
        return latest_episode
        
    def untrack_series(self):
        # Find the MainWindow instance
        parent = self
        while parent is not None:
            if isinstance(parent, MainWindow):
                main_window = parent
                break
            parent = parent.parent()
        
        if not main_window:
            return
            
        # Remove series from tracked anime
        self.manager.tracked_anime = [
            anime for anime in self.manager.tracked_anime
            if self.series_name not in anime
        ]
        self.manager.save_tracked_anime(self.manager.tracked_anime)
        
        # Update all UI elements
        QTimer.singleShot(0, main_window.update_tracked_list)  # Update tracked list
        QTimer.singleShot(100, main_window.refresh_card_statuses)  # Refresh all card statuses
        QTimer.singleShot(200, lambda: main_window.grid_layout.update())  # Force grid layout update
        
        # Show success message
        QMessageBox.information(self, "Success", f"Stopped tracking: {self.series_name}")
        
    def update_status(self):
        status = self.check_series_status()
        is_tracked = any(self.series_name in anime for anime in self.manager.tracked_anime)
        
        if status == "Ended":
            self.status_label.setText("Series Ended ✓")
            self.status_label.setStyleSheet("color: #d63031; font-weight: bold;")
            self.end_notice.setText("Series has finished airing.\nClick to remove from tracking.")
            self.end_notice.show()
            self.setProperty("ended", True)
        else:
            self.status_label.setText("✓ Tracking" if is_tracked else "Click to Track")
            self.status_label.setStyleSheet(
                "color: #00b894; font-weight: bold;" if is_tracked else "color: #0984e3;"
            )
            self.end_notice.hide()
            self.setProperty("ended", False)
        
        # Force style update
        self.style().unpolish(self)
        self.style().polish(self)
        
    def check_series_status(self):
        try:
            import requests
            import time
            
            # Clean up title for search
            search_title = self.series_name.replace("[SubsPlease]", "").strip()
            
            # Search for anime
            response = requests.get(
                f"https://api.jikan.moe/v4/anime",
                params={"q": search_title, "limit": 1},
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                if data["data"]:
                    anime = data["data"][0]
                    status = anime.get("status", "").lower()
                    if "finished" in status or "completed" in status:
                        return "Ended"
            
            return "Ongoing"
            
        except Exception:
            return "Ongoing"  # Default to ongoing if check fails
        
    def update_countdown(self):
        try:
            schedule_data = self.manager.fetch_schedule()
            if not schedule_data:
                self.countdown_label.setText("Schedule unavailable")
                return
                
            current_time = datetime.utcnow()
            next_time = None
            
            for day, shows in schedule_data["schedule"].items():
                for show in shows:
                    if self.series_name in show['title']:
                        time_str = show['time']
                        anime_time = datetime.strptime(time_str, "%H:%M")
                        
                        # Get day index (0 = Monday, 6 = Sunday)
                        day_index = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"].index(day.lower())
                        
                        # Get current day index
                        current_day_index = current_time.weekday()
                        
                        # Calculate days until next episode
                        days_until = day_index - current_day_index
                        if days_until <= 0:  # If day has passed this week, add 7 days
                            days_until += 7
                            
                        # Set the date for the next episode
                        anime_time = anime_time.replace(
                            year=current_time.year,
                            month=current_time.month,
                            day=current_time.day + days_until
                        )
                        
                        if next_time is None or anime_time < next_time:
                            next_time = anime_time
            
            if next_time:
                time_until = next_time - current_time
                days = time_until.days
                hours = int((time_until.total_seconds() % (24 * 3600)) // 3600)
                minutes = int((time_until.total_seconds() % 3600) // 60)
                
                countdown_text = "Next episode in: "
                if days > 0:
                    countdown_text += f"{days}d "
                countdown_text += f"{hours}h {minutes}m"
                
                self.countdown_label.setText(countdown_text)
                self.countdown_label.show()
            else:
                self.countdown_label.setText("No scheduled episodes found")
                
        except Exception as e:
            print(f"Error updating countdown: {str(e)}")
            self.countdown_label.setText("Schedule unavailable")

class DownloadCard(FlippableCard):
    def __init__(self, filename, manager, parent=None):
        super().__init__()
        self.filename = filename
        self.manager = manager
        self.original_pixmap = None
        self.setup_ui()
        
        # Setup progress update timer
        self.progress_timer = QTimer()
        self.progress_timer.timeout.connect(self.update_progress)
        self.progress_timer.start(1000)  # Update every second
        
    def setup_ui(self):
        self.setObjectName("downloadCard")
        self.setStyleSheet("""
            #downloadCard {
                background-color: white;
                border-radius: 10px;
                border: 1px solid #e0e0e0;
            }
            #downloadCard:hover {
                border: 1px solid #007AFF;
            }
            QLabel {
                color: #333333;
            }
        """)
        
        # Main layout
        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(10, 10, 10, 10)
        self.layout.setSpacing(8)
        
        # Create front and back widgets in the same layout
        self.front_widget = QWidget()
        self.back_widget = QWidget()
        self.setup_front()
        self.setup_back()
        
        # Add both to layout
        self.layout.addWidget(self.front_widget)
        self.layout.addWidget(self.back_widget)
        self.back_widget.hide()
        
        # Load image
        series_name = self.filename.replace("[SubsPlease]", "").strip().split(" - ")[0]
        self.loader = ImageLoader(series_name, self.manager)
        self.loader.image_loaded.connect(self.set_image)
        self.loader.start()
        
    def setup_front(self):
        layout = QVBoxLayout(self.front_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(8)
        
        # Image
        self.image_label = QLabel()
        self.image_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.image_label.setFixedSize(200, 280)  # Fixed size for stability
        layout.addWidget(self.image_label)
        
        # Series name
        series_name = self.filename.replace("[SubsPlease]", "").strip().split(" - ")[0]
        name_label = QLabel(series_name)
        name_label.setWordWrap(True)
        name_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        name_label.setStyleSheet("font-weight: bold; font-size: 14px;")
        layout.addWidget(name_label)
        
        # Episode info
        episode_info = self.filename.split(" - ")[-1].split("[")[0].strip()
        episode_label = QLabel(f"Episode {episode_info}")
        episode_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        episode_label.setStyleSheet("color: #666666;")
        layout.addWidget(episode_label)
        
        # Progress bar for downloading episodes
        self.progress_bar = QProgressBar()
        self.progress_bar.setStyleSheet("""
            QProgressBar {
                border: 1px solid #e0e0e0;
                border-radius: 5px;
                text-align: center;
                background-color: #f5f5f7;
            }
            QProgressBar::chunk {
                background-color: #007AFF;
                border-radius: 5px;
            }
        """)
        self.progress_bar.hide()
        layout.addWidget(self.progress_bar)
        
        # Check if this episode is downloading
        if self.manager.qb_client:
            torrents = self.manager.qb_client.torrents_info()
            for torrent in torrents:
                if self.filename in torrent.content_path:
                    self.progress_bar.show()
                    self.progress_bar.setValue(int(torrent.progress * 100))
                    break

    def setup_back(self):
        layout = QVBoxLayout(self.back_widget)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(10)
        
        # Info container
        info_frame = QFrame()
        info_frame.setStyleSheet("""
            QFrame {
                background-color: #f5f5f7;
                border-radius: 10px;
                padding: 15px;
            }
        """)
        info_layout = QVBoxLayout(info_frame)
        
        # Full filename
        name_label = QLabel(self.filename)
        name_label.setStyleSheet("font-weight: bold; font-size: 14px;")
        name_label.setWordWrap(True)
        info_layout.addWidget(name_label)
        
        # File info
        try:
            file_path = os.path.join(self.manager.settings['download_folder'], self.filename)
            size_mb = os.path.getsize(file_path) / (1024 * 1024)
            info_label = QLabel(f"Size: {size_mb:.1f} MB")
            info_label.setStyleSheet("color: #666666;")
            info_layout.addWidget(info_label)
        except:
            pass
        
        layout.addWidget(info_frame)
        
        # Delete button
        delete_btn = QPushButton("Delete Episode")
        delete_btn.setStyleSheet("""
            QPushButton {
                background-color: #ff3b30;
                color: white;
                border-radius: 5px;
                padding: 8px;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #ff453a;
            }
        """)
        delete_btn.clicked.connect(self.delete_episode)
        layout.addWidget(delete_btn)
        
    def set_image(self, title, pixmap):
        series_name = self.filename.replace("[SubsPlease]", "").strip().split(" - ")[0]
        if title == series_name:
            self.original_pixmap = pixmap
            scaled_pixmap = pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def resizeEvent(self, event):
        super().resizeEvent(event)
        if self.original_pixmap:
            scaled_pixmap = self.original_pixmap.scaled(
                self.image_label.size(),
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self.image_label.setPixmap(scaled_pixmap)
            
    def mousePressEvent(self, event):
        self.flip_card()
        
    def delete_episode(self):
        reply = QMessageBox.question(
            self, 'Delete Episode',
            f'Are you sure you want to delete {self.filename}?',
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No
        )
        
        if reply == QMessageBox.StandardButton.Yes:
            try:
                # Delete local file
                file_path = os.path.join(self.manager.settings['download_folder'], self.filename)
                os.remove(file_path)
                
                # Remove from qBittorrent if it exists
                if self.manager.qb_client:
                    torrents = self.manager.qb_client.torrents_info()
                    for torrent in torrents:
                        if self.filename in torrent.content_path:
                            self.manager.qb_client.torrents_delete(
                                delete_files=True, 
                                torrent_hashes=torrent.hash
                            )
                
                # Find MainWindow and update
                parent = self
                while parent is not None:
                    if isinstance(parent, MainWindow):
                        parent.update_downloads_list()
                        break
                    parent = parent.parent()
                    
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Failed to delete file: {str(e)}")

    def update_progress(self):
        if self.manager.qb_client:
            torrents = self.manager.qb_client.torrents_info()
            for torrent in torrents:
                if self.filename in torrent.content_path:
                    self.progress_bar.show()
                    self.progress_bar.setValue(int(torrent.progress * 100))
                    return
            # If we get here, torrent not found (completed or removed)
            self.progress_bar.hide()

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        print("Initializing ANICHAIN...")
        self.manager = AnimeManager()
        self.setWindowTitle("ANICHAIN")
        self.setMinimumSize(800, 600)
        
        # Cache for series status
        self.series_status_cache = {}
        
        # Create central widget with layout
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)
        
        # Setup UI components first
        print("Setting up UI components...")
        self.setup_ui()
        
        # Add status bar for qBittorrent at the bottom
        self.setup_status_bar()
        
        # Setup timers
        self.setup_timers()
        
        # Initialize resize timer
        self.resize_timer = QTimer()
        self.resize_timer.setSingleShot(True)
        self.resize_timer.timeout.connect(self.handle_resize_timeout)
        
        # Load initial data asynchronously with longer delays
        print("Starting asynchronous data loading...")
        QTimer.singleShot(0, self.connect_qbittorrent)
        QTimer.singleShot(500, self.load_feed)
        QTimer.singleShot(1000, self.load_schedule)
        QTimer.singleShot(1500, self.update_tracked_list)
        QTimer.singleShot(2000, self.update_downloads_list)
        
    def connect_qbittorrent(self):
        """Try to connect to qBittorrent first"""
        print("Connecting to qBittorrent...")
        if not self.manager.setup_qbittorrent():
            print("qBittorrent connection failed, showing dialog...")
            QTimer.singleShot(0, self.show_qbittorrent_dialog)
        else:
            print("qBittorrent connected successfully")
            self.update_qbittorrent_status()
            
    def load_feed(self):
        """Load RSS feed asynchronously"""
        print("Loading RSS feed...")
        try:
            self.manager.load_feed()
            print("RSS feed loaded successfully")
            self.display_anime_tiles()
        except Exception as e:
            print(f"Error loading RSS feed: {str(e)}")
            
    def load_schedule(self):
        """Load schedule asynchronously"""
        print("Loading schedule...")
        try:
            self.manager.load_schedule()
            print("Schedule loaded successfully")
            self.display_schedule()
        except Exception as e:
            print(f"Error loading schedule: {str(e)}")
            
    def update_tracked_list(self):
        """Update tracked anime list asynchronously"""
        print("Updating tracked anime list...")
        try:
            self.display_tracked_anime()
            print("Tracked anime list updated")
        except Exception as e:
            print(f"Error updating tracked list: {str(e)}")
            
    def update_downloads_list(self):
        """Update downloads list asynchronously"""
        print("Updating downloads list...")
        try:
            self.display_downloads()
            print("Downloads list updated")
        except Exception as e:
            print(f"Error updating downloads list: {str(e)}")
            
    def handle_resize_timeout(self):
        """Handle resize event after timer timeout"""
        try:
            self.display_anime_tiles()
        except Exception as e:
            print(f"Error handling resize: {str(e)}")

    def check_series_status(self, series_name):
        """Check series status with caching"""
        if series_name in self.series_status_cache:
            return self.series_status_cache[series_name]
            
        try:
            import requests
            import time
            
            # Clean up title for search
            search_title = series_name.replace("[SubsPlease]", "").strip()
            
            # Search for anime
            response = requests.get(
                f"https://api.jikan.moe/v4/anime",
                params={"q": search_title, "limit": 1},
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                if data["data"]:
                    anime = data["data"][0]
                    status = anime.get("status", "").lower()
                    result = "Ended" if "finished" in status or "completed" in status else "Ongoing"
                    self.series_status_cache[series_name] = result
                    return result
            
            return "Ongoing"
            
        except Exception:
            return "Ongoing"  # Default to ongoing if check fails
            
    def display_anime_tiles(self):
        """Display anime tiles in the grid layout"""
        # Clear existing items
        for i in reversed(range(self.grid_layout.count())): 
            self.grid_layout.itemAt(i).widget().setParent(None)
            
        # Determine number of columns based on window width
        window_width = self.width()
        if window_width >= 1600:
            columns = 7  # Large window
        elif window_width >= 1200:
            columns = 5  # Medium window
        else:
            columns = 3  # Small window
            
        # Add new items
        for i, entry in enumerate(self.manager.fetch_rss_feed().entries):
            card = AnimeCard(entry.get("title", "No Title"), self.manager)
            card.clicked.connect(self.on_anime_clicked)
            card.setFixedSize(220, 380)  # Fixed size for entire card
            self.grid_layout.addWidget(card, i // columns, i % columns)
            
    def resizeEvent(self, event):
        """Handle window resize events"""
        super().resizeEvent(event)
        # Reset and restart the timer
        self.resize_timer.stop()
        self.resize_timer.start(150)  # Wait for 150ms of no resize events
        
    def update_clock(self):
        """Update the clock display"""
        current_time = datetime.utcnow().strftime("%Y-%m-%d %H:%M:%S UTC")
        self.current_time_label.setText(f"Current Time: {current_time}")
        
    def setup_timers(self):
        """Setup all application timers"""
        # Add qBittorrent connection check every minute
        self.qb_timer = QTimer()
        self.qb_timer.timeout.connect(self.update_qbittorrent_status)
        self.qb_timer.start(60000)  # Check every minute
        
        # Resize timer for debouncing
        self.resize_timer = QTimer()
        self.resize_timer.setSingleShot(True)
        self.resize_timer.timeout.connect(self.handle_resize_timeout)
        
        # Clock timer
        self.clock_timer = QTimer()
        self.clock_timer.timeout.connect(self.update_clock)
        self.clock_timer.start(1000)
        
        # Schedule timer
        self.schedule_timer = QTimer()
        self.schedule_timer.timeout.connect(self.load_schedule)
        self.schedule_timer.start(300000)  # Every 5 minutes
        
        # Update downloads more frequently to show progress
        self.downloads_timer = QTimer()
        self.downloads_timer.timeout.connect(self.update_downloads_list)
        self.downloads_timer.start(5000)  # Every 5 seconds
        
        # Feed refresh timer
        self.feed_timer = QTimer()
        self.feed_timer.timeout.connect(self.load_feed)
        self.feed_timer.start(300000)  # Every 5 minutes
        
    def show_qbittorrent_dialog(self):
        """Show the qBittorrent connection dialog"""
        dialog = QBittorrentDialog(self.manager, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.update_qbittorrent_status()
            
    def update_qbittorrent_status(self):
        """Update the qBittorrent connection status display"""
        try:
            if self.manager.qb_client and self.manager.setup_qbittorrent():
                self.status_circle.setStyleSheet("color: #00b894; font-size: 14px; font-weight: bold;")
                self.status_text.setText("qBittorrent Connected")
                self.status_text.setStyleSheet("color: #00b894; font-weight: bold;")
                self.reconnect_btn.hide()
            else:
                self.status_circle.setStyleSheet("color: #ff3b30; font-size: 14px; font-weight: bold;")
                self.status_text.setText("qBittorrent Disconnected")
                self.status_text.setStyleSheet("color: #ff3b30; font-weight: bold;")
                self.reconnect_btn.show()
        except Exception as e:
            print(f"Error updating qBittorrent status: {str(e)}")
            self.status_circle.setStyleSheet("color: #ff3b30; font-size: 14px; font-weight: bold;")
            self.status_text.setText("qBittorrent Error")
            self.status_text.setStyleSheet("color: #ff3b30; font-weight: bold;")
            self.reconnect_btn.show()
            
    def ensure_qbittorrent_connection(self):
        """Ensure there is a working qBittorrent connection"""
        if not self.manager.qb_client or not self.manager.setup_qbittorrent():
            dialog = QBittorrentDialog(self.manager, self)
            if dialog.exec() == QDialog.DialogCode.Accepted:
                self.update_qbittorrent_status()
                return True
            return False
        return True

    def display_tracked_anime(self):
        """Display tracked anime in the grid layout"""
        # Clear existing items
        for i in reversed(range(self.tracked_layout.count())):
            self.tracked_layout.itemAt(i).widget().setParent(None)
        
        # Get unique series names from both tracked and downloaded
        series_set = set()
        
        # Add from tracked anime
        for anime in self.manager.tracked_anime:
            series_name = anime.replace("[SubsPlease]", "").strip().split(" - ")[0]
            series_set.add(series_name)
            
        # Add from downloaded files
        downloaded_files = self.manager.get_downloaded_files()
        for file in downloaded_files:
            if file.endswith(".mkv"):
                series_name = file.replace("[SubsPlease]", "").strip().split(" - ")[0]
                series_set.add(series_name)
        
        # Determine number of columns based on window width
        window_width = self.width()
        if window_width >= 1600:
            columns = 7  # Large window
        elif window_width >= 1200:
            columns = 5  # Medium window
        else:
            columns = 3  # Small window
        
        # Create cards for each series
        for i, series in enumerate(sorted(series_set)):
            card = TrackedAnimeCard(series, self.manager)
            card.setFixedSize(220, 380)  # Fixed size for entire card
            self.tracked_layout.addWidget(card, i // columns, i % columns)
            
    def display_schedule(self):
        """Display schedule in the schedule text area"""
        schedule_data = self.manager.fetch_schedule()
        if not schedule_data:
            self.schedule_text.setText("Failed to load schedule. Will retry in 5 minutes.")
            return
            
        current_time = datetime.utcnow()
        next_anime = None
        
        # Create a styled schedule layout
        schedule_html = """
        <style>
            .schedule-container {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
                padding: 20px;
            }
            .day-header {
                font-size: 18px;
                font-weight: bold;
                color: #333;
                margin: 15px 0 10px 0;
                padding-bottom: 5px;
                border-bottom: 2px solid #007AFF;
            }
            .show-item {
                padding: 8px 15px;
                margin: 5px 0;
                background-color: #f8f9fa;
                border-radius: 5px;
                font-size: 14px;
            }
            .show-item.next {
                background-color: #e3f2fd;
                border-left: 4px solid #007AFF;
            }
            .show-item.tracked {
                background-color: #e6fff7;
                border-left: 4px solid #00b894;
            }
            .time {
                color: #666;
                font-weight: 500;
            }
        </style>
        <div class='schedule-container'>
        """
        
        for day, shows in schedule_data["schedule"].items():
            schedule_html += f"<div class='day-header'>{day.capitalize()}</div>"
            
            for show in shows:
                time_str = show['time']
                title = show['title']
                
                # Check if show is being tracked
                is_tracked = any(title in anime for anime in self.manager.tracked_anime)
                
                anime_time = datetime.strptime(time_str, "%H:%M")
                anime_time = anime_time.replace(
                    year=current_time.year,
                    month=current_time.month,
                    day=current_time.day
                )
                
                is_next = anime_time > current_time and (next_anime is None or anime_time < next_anime[1])
                if is_next:
                    next_anime = (title, anime_time)
                
                # Determine class based on status
                classes = ['show-item']
                if is_next:
                    classes.append('next')
                if is_tracked:
                    classes.append('tracked')
                
                schedule_html += f"""
                <div class='{' '.join(classes)}'>
                    <span class='time'>{time_str} UTC</span> - {title}
                    {' (Next)' if is_next else ''}
                </div>
                """
        
        schedule_html += "</div>"
        self.schedule_text.setHtml(schedule_html)
        
        if next_anime:
            time_until = next_anime[1] - current_time
            hours = int(time_until.total_seconds() // 3600)
            minutes = int((time_until.total_seconds() % 3600) // 60)
            self.next_anime_label.setText(
                f"Next Episode: {next_anime[0]} at {next_anime[1].strftime('%H:%M UTC')} (in {hours}h {minutes}m)"
            )
        
    def update_tracked_list(self):
        """Update tracked anime list asynchronously"""
        print("Updating tracked anime list...")
        try:
            self.display_tracked_anime()
            print("Tracked anime list updated")
        except Exception as e:
            print(f"Error updating tracked list: {str(e)}")
            
    def update_downloads_list(self):
        """Update downloads list asynchronously"""
        print("Updating downloads list...")
        try:
            self.display_downloads()
            print("Downloads list updated")
        except Exception as e:
            print(f"Error updating downloads list: {str(e)}")
            
    def display_downloads(self):
        """Display downloads in the downloads layout"""
        # Clear existing items
        for i in reversed(range(self.downloads_layout.count())):
            self.downloads_layout.itemAt(i).widget().setParent(None)
        
        # Get .mkv files
        files = [f for f in self.manager.get_downloaded_files() if f.endswith('.mkv')]
        
        # Determine number of columns based on window width
        window_width = self.width()
        if window_width >= 1600:
            columns = 7  # Large window
        elif window_width >= 1200:
            columns = 5  # Medium window
        else:
            columns = 3  # Small window
        
        # Create cards for each file
        for i, filename in enumerate(sorted(files)):
            card = DownloadCard(filename, self.manager)
            card.setFixedSize(220, 380)  # Fixed size for entire card
            self.downloads_layout.addWidget(card, i // columns, i % columns)
        
    def browse_folder(self):
        folder = QFileDialog.getExistingDirectory(
            self, "Select Download Folder",
            self.manager.settings["download_folder"]
        )
        if folder:
            self.folder_entry.setText(folder)
            
    def save_settings(self):
        new_settings = {
            "download_folder": self.folder_entry.text(),
            "rss_url": self.rss_entry.text(),
            "qb_host": self.qb_host_entry.text(),
            "qb_username": self.qb_username_entry.text(),
            "qb_password": self.qb_password_entry.text()
        }
        
        try:
            self.manager.save_settings(new_settings)
            QMessageBox.information(self, "Success", "Settings saved successfully")
            
            # Try to reconnect to qBittorrent if settings changed
            if (new_settings["qb_host"] != self.manager.settings["qb_host"] or
                new_settings["qb_username"] != self.manager.settings["qb_username"] or
                new_settings["qb_password"] != self.manager.settings["qb_password"]):
                if self.manager.setup_qbittorrent():
                    self.qb_status_label.setText("qBittorrent: Connected ✓")
                    self.qb_status_label.setStyleSheet("color: green; font-weight: bold;")
                else:
                    self.qb_status_label.setText("qBittorrent: Disconnected ✗")
                    self.qb_status_label.setStyleSheet("color: red; font-weight: bold;")
                    
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Failed to save settings: {str(e)}")
        
    def perform_search(self):
        search_term = self.search_input.text().lower()
        if not search_term:
            self.load_feed()  # Reset to show all anime
            return
            
        feed = self.manager.fetch_rss_feed()
        if feed:
            filtered_entries = [
                entry for entry in feed.entries
                if search_term in entry.get("title", "").lower()
            ]
            self.display_anime_tiles(filtered_entries)
            
    def refresh_card_statuses(self):
        for i in range(self.grid_layout.count()):
            card = self.grid_layout.itemAt(i).widget()
            if isinstance(card, AnimeCard):
                card.update_status()
                
    def remove_tracked(self):
        # Get the selected widget from tracked layout
        for i in range(self.tracked_layout.count()):
            widget = self.tracked_layout.itemAt(i).widget()
            if isinstance(widget, TrackedAnimeCard) and widget.isActiveWindow():
                series_name = widget.series_name
                # Remove all episodes of this series from tracked_anime
                self.manager.tracked_anime = [
                    anime for anime in self.manager.tracked_anime
                    if series_name not in anime
                ]
                self.manager.save_tracked_anime(self.manager.tracked_anime)
                self.update_tracked_list()
                self.refresh_card_statuses()
                return

    def setup_status_bar(self):
        """Setup the qBittorrent status bar"""
        self.statusBar().setFixedHeight(30)
        self.statusBar().setStyleSheet("""
            QStatusBar {
                background-color: white;
                border-top: 1px solid #e0e0e0;
            }
        """)
        
        # Create a widget for status items
        status_widget = QWidget()
        status_layout = QHBoxLayout(status_widget)
        status_layout.setContentsMargins(20, 0, 20, 0)
        
        # Status indicator circle
        self.status_circle = QLabel("●")
        self.status_circle.setFixedWidth(20)
        
        # Status text
        self.status_text = QLabel()
        self.status_text.setStyleSheet("font-size: 12px;")
        
        # Reconnect button
        self.reconnect_btn = QPushButton("Reconnect")
        self.reconnect_btn.setStyleSheet("""
            QPushButton {
                background-color: #ff3b30;
                color: white;
                border-radius: 3px;
                padding: 3px 8px;
                font-size: 12px;
            }
            QPushButton:hover {
                background-color: #ff453a;
            }
        """)
        self.reconnect_btn.clicked.connect(self.show_qbittorrent_dialog)
        self.reconnect_btn.hide()
        
        status_layout.addStretch()
        status_layout.addWidget(self.status_circle)
        status_layout.addWidget(self.status_text)
        status_layout.addWidget(self.reconnect_btn)
        
        # Add the status widget to the status bar
        self.statusBar().addPermanentWidget(status_widget)
        
    def on_anime_clicked(self, title):
        """Handle anime card click with optimized tracking"""
        # Extract series name
        series_name = title.replace("[SubsPlease]", "").strip().split(" - ")[0]
        
        # Check if already tracking
        is_tracked = any(series_name in anime for anime in self.manager.tracked_anime)
        
        if is_tracked:
            # Just untrack series without deleting files
            self.manager.tracked_anime = [
                anime for anime in self.manager.tracked_anime
                if series_name not in anime
            ]
            self.manager.save_tracked_anime(self.manager.tracked_anime)
            
            # Update UI safely
            QTimer.singleShot(0, lambda: self.update_tracked_list())
            QTimer.singleShot(100, lambda: self.refresh_card_statuses())
            QMessageBox.information(self, "Success", f"Stopped tracking: {series_name}")
            return
            
        # If not tracked, proceed with tracking and downloading
        feed = self.manager.fetch_rss_feed()
        if not feed:
            QMessageBox.warning(self, "Error", "Could not fetch RSS feed")
            return
            
        for entry in feed.entries:
            if entry.get("title") == title:
                if not self.manager.qb_client:
                    if not self.ensure_qbittorrent_connection():
                        QMessageBox.critical(self, "Error", "Not connected to qBittorrent")
                        return
                
                # Add torrent with category
                if self.manager.add_torrent(entry.get("link"), category="Anime"):
                    self.manager.tracked_anime.append(series_name)
                    self.manager.save_tracked_anime(self.manager.tracked_anime)
                    
                    # Update UI safely using timers
                    QTimer.singleShot(0, lambda: self.update_tracked_list())
                    QTimer.singleShot(100, lambda: self.refresh_card_statuses())
                    QMessageBox.information(self, "Success", 
                        f"Started downloading: {title}\nTracking series: {series_name}")
                else:
                    QMessageBox.warning(self, "Error", 
                        f"Failed to start download for: {title}")
                return
                
        QMessageBox.warning(self, "Error", f"Could not find torrent link for: {title}")

    def setup_ui(self):
        """Setup the main UI components"""
        # Top navigation bar
        nav_bar = QWidget()
        nav_bar.setStyleSheet("""
            QWidget {
                background-color: #ffffff;
                border-bottom: 1px solid #e0e0e0;
            }
        """)
        nav_layout = QHBoxLayout(nav_bar)
        nav_layout.setContentsMargins(20, 10, 20, 10)
        nav_layout.setSpacing(20)
        
        # Logo
        logo = QLabel("ANICHAIN")
        logo.setStyleSheet("""
            QLabel {
                color: #333333;
                font-size: 24px;
                font-weight: bold;
            }
        """)
        nav_layout.addWidget(logo)
        
        # Navigation buttons
        nav_buttons = ["Available", "Schedule", "Tracked", "Downloads", "Settings"]
        self.nav_button_group = QButtonGroup(self)
        self.nav_button_group.buttonClicked.connect(self.handle_nav_click)
        
        for text in nav_buttons:
            btn = QPushButton(text)
            btn.setCheckable(True)
            btn.setStyleSheet("""
                QPushButton {
                    color: #666666;
                    border: none;
                    padding: 8px 16px;
                    font-size: 14px;
                    background: transparent;
                }
                QPushButton:hover {
                    color: #333333;
                }
                QPushButton:checked {
                    color: #007AFF;
                    font-weight: bold;
                }
            """)
            nav_layout.addWidget(btn)
            self.nav_button_group.addButton(btn)
        
        # Search bar
        search_frame = QFrame()
        search_frame.setStyleSheet("""
            QFrame {
                background-color: #f5f5f7;
                border-radius: 20px;
                padding: 5px;
            }
        """)
        search_layout = QHBoxLayout(search_frame)
        search_layout.setContentsMargins(15, 5, 15, 5)
        
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search anime")
        self.search_input.setStyleSheet("""
            QLineEdit {
                border: none;
                background: transparent;
                color: #333333;
                font-size: 14px;
            }
        """)
        self.search_input.returnPressed.connect(self.perform_search)
        search_layout.addWidget(self.search_input)
        
        search_btn = QPushButton("Search")
        search_btn.clicked.connect(self.perform_search)
        search_btn.setStyleSheet("""
            QPushButton {
                background-color: #007AFF;
                color: white;
                border-radius: 15px;
                padding: 8px 20px;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #0066CC;
            }
        """)
        nav_layout.addWidget(search_frame)
        nav_layout.addWidget(search_btn)
        
        # Add navigation bar to main layout
        main_layout = self.centralWidget().layout()
        main_layout.addWidget(nav_bar)
        
        # Content area
        content_widget = QWidget()
        content_layout = QVBoxLayout(content_widget)
        content_layout.setContentsMargins(20, 20, 20, 20)
        content_layout.setSpacing(20)
        
        # Create stacked widget for content
        self.content_stack = QStackedWidget()
        content_layout.addWidget(self.content_stack)
        
        # Add pages
        self.setup_anime_page()
        self.setup_schedule_page()
        self.setup_tracked_page()
        self.setup_downloads_page()
        self.setup_settings_page()
        
        main_layout.addWidget(content_widget)
        
    def handle_nav_click(self, button):
        """Handle navigation button clicks"""
        index = self.nav_button_group.buttons().index(button)
        self.content_stack.setCurrentIndex(index)
        
    def setup_anime_page(self):
        """Setup the available anime page"""
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(0, 0, 0, 0)
        
        # Scroll area for anime grid
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet("""
            QScrollArea {
                border: none;
                background-color: #ffffff;
            }
            QScrollBar:vertical {
                border: none;
                background: #f5f5f7;
                width: 10px;
                margin: 0px;
            }
            QScrollBar::handle:vertical {
                background: #c1c1c1;
                min-height: 30px;
                border-radius: 5px;
            }
            QScrollBar::handle:vertical:hover {
                background: #a8a8a8;
            }
        """)
        
        self.grid_widget = QWidget()
        self.grid_layout = QGridLayout(self.grid_widget)
        self.grid_layout.setSpacing(20)
        scroll.setWidget(self.grid_widget)
        
        layout.addWidget(scroll)
        self.content_stack.addWidget(page)
        
    def setup_schedule_page(self):
        """Setup the schedule page"""
        schedule_page = QWidget()
        layout = QVBoxLayout(schedule_page)
        layout.setContentsMargins(0, 0, 0, 0)
        
        # Header with current time
        self.current_time_label = QLabel()
        self.current_time_label.setStyleSheet("""
            QLabel {
                color: #333333;
                font-weight: bold;
                font-size: 14px;
            }
        """)
        layout.addWidget(self.current_time_label)
        
        # Next anime label
        self.next_anime_label = QLabel()
        self.next_anime_label.setStyleSheet("""
            QLabel {
                color: #333333;
                font-weight: bold;
                font-size: 14px;
            }
        """)
        layout.addWidget(self.next_anime_label)
        
        # Schedule text area
        self.schedule_text = QTextEdit()
        self.schedule_text.setReadOnly(True)
        self.schedule_text.setStyleSheet("""
            QTextEdit {
                background-color: white;
                border: 1px solid #e0e0e0;
                border-radius: 10px;
                padding: 15px;
                color: #333333;
                font-size: 14px;
            }
        """)
        layout.addWidget(self.schedule_text)
        
        self.content_stack.addWidget(schedule_page)
        
    def setup_tracked_page(self):
        """Setup the tracked anime page"""
        tracked_page = QWidget()
        layout = QVBoxLayout(tracked_page)
        layout.setContentsMargins(0, 0, 0, 0)
        
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet("""
            QScrollArea {
                border: none;
                background-color: white;
            }
        """)
        
        tracked_widget = QWidget()
        self.tracked_layout = QGridLayout(tracked_widget)
        self.tracked_layout.setSpacing(20)
        scroll.setWidget(tracked_widget)
        
        layout.addWidget(scroll)
        self.content_stack.addWidget(tracked_page)
        
    def setup_downloads_page(self):
        """Setup the downloads page"""
        downloads_page = QWidget()
        layout = QVBoxLayout(downloads_page)
        layout.setContentsMargins(0, 0, 0, 0)
        
        self.folder_label = QLabel(f"Download Folder: {self.manager.settings['download_folder']}")
        self.folder_label.setStyleSheet("""
            QLabel {
                color: #333333;
                font-weight: bold;
                font-size: 14px;
                margin-bottom: 10px;
            }
        """)
        layout.addWidget(self.folder_label)
        
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet("""
            QScrollArea {
                border: none;
                background-color: white;
            }
        """)
        
        downloads_widget = QWidget()
        self.downloads_layout = QGridLayout(downloads_widget)
        self.downloads_layout.setSpacing(20)
        scroll.setWidget(downloads_widget)
        
        layout.addWidget(scroll)
        self.content_stack.addWidget(downloads_page)
        
    def setup_settings_page(self):
        """Setup the settings page"""
        settings_page = QWidget()
        layout = QVBoxLayout(settings_page)
        layout.setContentsMargins(0, 0, 0, 0)
        
        settings_frame = QFrame()
        settings_frame.setStyleSheet("""
            QFrame {
                background-color: white;
                border: 1px solid #e0e0e0;
                border-radius: 10px;
                padding: 20px;
            }
        """)
        settings_layout = QVBoxLayout(settings_frame)
        
        # Download folder
        settings_layout.addWidget(QLabel("Download Folder:"))
        folder_frame = QWidget()
        folder_layout = QHBoxLayout(folder_frame)
        folder_layout.setContentsMargins(0, 0, 0, 0)
        
        self.folder_entry = QLineEdit(self.manager.settings["download_folder"])
        folder_layout.addWidget(self.folder_entry)
        
        browse_button = QPushButton("Browse")
        browse_button.clicked.connect(self.browse_folder)
        folder_layout.addWidget(browse_button)
        settings_layout.addWidget(folder_frame)
        
        # RSS URL
        settings_layout.addWidget(QLabel("RSS URL:"))
        self.rss_entry = QLineEdit(self.manager.settings["rss_url"])
        settings_layout.addWidget(self.rss_entry)
        
        # qBittorrent settings
        settings_layout.addWidget(QLabel("qBittorrent Settings"))
        
        settings_layout.addWidget(QLabel("Host:"))
        self.qb_host_entry = QLineEdit(self.manager.settings["qb_host"])
        settings_layout.addWidget(self.qb_host_entry)
        
        settings_layout.addWidget(QLabel("Username:"))
        self.qb_username_entry = QLineEdit(self.manager.settings["qb_username"])
        settings_layout.addWidget(self.qb_username_entry)
        
        settings_layout.addWidget(QLabel("Password:"))
        self.qb_password_entry = QLineEdit(self.manager.settings["qb_password"])
        self.qb_password_entry.setEchoMode(QLineEdit.EchoMode.Password)
        settings_layout.addWidget(self.qb_password_entry)
        
        # Save button
        save_button = QPushButton("Save Settings")
        save_button.clicked.connect(self.save_settings)
        save_button.setStyleSheet("""
            QPushButton {
                background-color: #007AFF;
                color: white;
                border-radius: 5px;
                padding: 8px 16px;
                font-size: 14px;
            }
            QPushButton:hover {
                background-color: #0066CC;
            }
        """)
        settings_layout.addWidget(save_button, alignment=Qt.AlignmentFlag.AlignRight)
        
        layout.addWidget(settings_frame)
        layout.addStretch()
        self.content_stack.addWidget(settings_page)

    def show_qbittorrent_dialog(self):
        """Show the qBittorrent connection dialog"""
        dialog = QBittorrentDialog(self.manager, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.update_qbittorrent_status()
            
    def update_qbittorrent_status(self):
        """Update the qBittorrent connection status display"""
        try:
            if self.manager.qb_client and self.manager.setup_qbittorrent():
                self.status_circle.setStyleSheet("color: #00b894; font-size: 14px; font-weight: bold;")
                self.status_text.setText("qBittorrent Connected")
                self.status_text.setStyleSheet("color: #00b894; font-weight: bold;")
                self.reconnect_btn.hide()
            else:
                self.status_circle.setStyleSheet("color: #ff3b30; font-size: 14px; font-weight: bold;")
                self.status_text.setText("qBittorrent Disconnected")
                self.status_text.setStyleSheet("color: #ff3b30; font-weight: bold;")
                self.reconnect_btn.show()
        except Exception as e:
            print(f"Error updating qBittorrent status: {str(e)}")
            self.status_circle.setStyleSheet("color: #ff3b30; font-size: 14px; font-weight: bold;")
            self.status_text.setText("qBittorrent Error")
            self.status_text.setStyleSheet("color: #ff3b30; font-weight: bold;")
            self.reconnect_btn.show()
            
    def ensure_qbittorrent_connection(self):
        """Ensure there is a working qBittorrent connection"""
        if not self.manager.qb_client or not self.manager.setup_qbittorrent():
            dialog = QBittorrentDialog(self.manager, self)
            if dialog.exec() == QDialog.DialogCode.Accepted:
                self.update_qbittorrent_status()
                return True
            return False
        return True

    def delete_episode(self, filename):
        """Delete an episode and remove it from qBittorrent"""
        try:
            # Remove from qBittorrent if it's in the queue
            if self.manager.qb_client:
                torrents = self.manager.qb_client.torrents_info()
                for torrent in torrents:
                    if filename in torrent.content_path:
                        self.manager.qb_client.torrents_delete(
                            delete_files=True,
                            torrent_hashes=torrent.hash
                        )
                        break
            
            # Delete the file
            file_path = os.path.join(self.manager.settings["download_folder"], filename)
            if os.path.exists(file_path):
                os.remove(file_path)
                
            # Update the downloads list
            self.update_downloads_list()
            
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Failed to delete file: {str(e)}")
            
    def update_progress(self):
        """Update download progress for all cards"""
        if not self.manager.qb_client:
            return
            
        try:
            torrents = self.manager.qb_client.torrents_info()
            for i in range(self.downloads_layout.count()):
                card = self.downloads_layout.itemAt(i).widget()
                if isinstance(card, DownloadCard):
                    for torrent in torrents:
                        if card.filename in torrent.content_path:
                            card.progress_bar.setValue(int(torrent.progress * 100))
                            card.progress_bar.show()
                            break
                            
        except Exception as e:
            print(f"Error updating progress: {str(e)}")
            
    def closeEvent(self, event):
        """Handle application close event"""
        # Stop all timers
        self.qb_timer.stop()
        self.resize_timer.stop()
        self.clock_timer.stop()
        self.schedule_timer.stop()
        self.downloads_timer.stop()
        self.feed_timer.stop()
        
        # Accept the close event
        event.accept()

def main():
    app = QApplication(sys.argv)
    
    # Set application style
    app.setStyle("Fusion")
    
    # Set color scheme
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor("#f5f5f7"))
    palette.setColor(QPalette.ColorRole.WindowText, QColor("#333333"))
    app.setPalette(palette)
    
    # Set font
    font = QFont(".AppleSystemUIFont", 10)  # Use system font
    app.setFont(font)
    
    # Set up signal handling for Ctrl+C
    import signal
    def signal_handler(signum, frame):
        print("\nClosing application...")
        app.quit()
    
    # Register the signal handler
    signal.signal(signal.SIGINT, signal_handler)
    
    # Create and show the main window
    window = MainWindow()
    window.show()
    
    # Create a timer to process signals
    timer = QTimer()
    timer.timeout.connect(lambda: None)  # Let Python process events
    timer.start(200)  # Check every 200ms
    
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
