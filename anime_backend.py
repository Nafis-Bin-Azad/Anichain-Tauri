import os
import json
import time
import queue
import threading
import requests
import feedparser
from datetime import datetime
from qbittorrentapi import Client

SETTINGS_FILE = "settings.txt"
TRACKED_FILE = "tracked_anime.txt"
PLACEHOLDER_IMAGE = "placeholder.jpg"
CACHE_DIR = "image_cache"
MAX_RETRIES = 3
JIKAN_RATE_LIMIT = 1

# Create cache directory if it doesn't exist
os.makedirs(CACHE_DIR, exist_ok=True)

class RateLimiter:
    def __init__(self, calls_per_second=1):
        self.calls_per_second = calls_per_second
        self.last_call = 0
        self.lock = threading.Lock()

    def wait(self):
        with self.lock:
            current_time = time.time()
            time_since_last_call = current_time - self.last_call
            if time_since_last_call < 1.0 / self.calls_per_second:
                time.sleep(1.0 / self.calls_per_second - time_since_last_call)
            self.last_call = time.time()

class AnimeManager:
    def __init__(self):
        self.settings = self.load_settings()
        self.tracked_anime = self.load_tracked_anime()
        self.qb_client = None
        self.jikan_limiter = RateLimiter(JIKAN_RATE_LIMIT)

    def load_settings(self):
        if os.path.exists(SETTINGS_FILE):
            with open(SETTINGS_FILE, "r") as f:
                return json.load(f)
        return {
            "download_folder": os.getcwd(),
            "rss_url": "https://subsplease.org/rss/?r=1080",
            "qb_host": "http://127.0.0.1:8080",
            "qb_username": "admin",
            "qb_password": "adminadmin"
        }

    def save_settings(self, settings):
        with open(SETTINGS_FILE, "w") as f:
            json.dump(settings, f)
        self.settings = settings

    def load_tracked_anime(self):
        if os.path.exists(TRACKED_FILE):
            with open(TRACKED_FILE, "r") as f:
                return [line.strip() for line in f.readlines()]
        return []

    def save_tracked_anime(self, tracked):
        with open(TRACKED_FILE, "w") as f:
            f.write("\n".join(tracked))
        self.tracked_anime = tracked

    def get_cached_image_path(self, title):
        safe_title = "".join(c for c in title if c.isalnum() or c in (' ', '-', '_')).rstrip()
        return os.path.join(CACHE_DIR, f"{safe_title}.jpg")

    def fetch_anime_image(self, title):
        # Clean up the title
        clean_title = title.replace("[SubsPlease]", "").strip()
        clean_title = clean_title.split(" - ")[0].strip()
        clean_title = clean_title.split("[")[0].strip()

        # Check cache
        cache_path = self.get_cached_image_path(clean_title)
        if os.path.exists(cache_path):
            print(f"Using cached image for {clean_title}")
            return cache_path

        for attempt in range(MAX_RETRIES):
            try:
                self.jikan_limiter.wait()
                query_url = f"https://api.jikan.moe/v4/anime?q={clean_title}&limit=1"
                response = requests.get(query_url)
                response.raise_for_status()
                data = response.json()
                
                if data.get("data") and data["data"]:
                    image_url = data["data"][0]["images"]["jpg"]["large_image_url"]
                    if image_url:
                        img_response = requests.get(image_url)
                        img_response.raise_for_status()
                        
                        with open(cache_path, 'wb') as f:
                            f.write(img_response.content)
                        
                        print(f"Successfully cached image for {clean_title}")
                        return cache_path
                
                if attempt < MAX_RETRIES - 1:
                    time.sleep(1)
                    
            except Exception as e:
                print(f"Error fetching image for {clean_title} (attempt {attempt + 1}/{MAX_RETRIES}): {e}")
                if attempt < MAX_RETRIES - 1:
                    time.sleep(1)
        
        return PLACEHOLDER_IMAGE

    def fetch_rss_feed(self):
        try:
            feed = feedparser.parse(self.settings["rss_url"])
            return feed
        except Exception as e:
            print(f"Error fetching RSS feed: {e}")
            return None

    def fetch_schedule(self):
        try:
            response = requests.get("https://subsplease.org/api/?f=schedule&tz=UTC")
            response.raise_for_status()
            return response.json()
        except Exception as e:
            print(f"Error fetching schedule: {e}")
            return None

    def setup_qbittorrent(self):
        try:
            self.qb_client = Client(
                host=self.settings["qb_host"],
                username=self.settings["qb_username"],
                password=self.settings["qb_password"]
            )
            self.qb_client.auth_log_in()
            return True
        except Exception as e:
            print(f"Failed to connect to qBittorrent: {e}")
            return False

    def add_torrent(self, link, category=None):
        """Add a torrent to qBittorrent with optional category."""
        try:
            if not self.qb_client:
                return False
            
            # Add torrent with category
            self.qb_client.torrents_add(
                urls=[link],
                category="Anime" if category else None
            )
            return True
        except Exception as e:
            print(f"Failed to add torrent: {str(e)}")
            return False

    def get_downloaded_files(self):
        if not os.path.exists(self.settings["download_folder"]):
            return []
            
        files = []
        for file in os.listdir(self.settings["download_folder"]):
            if os.path.isfile(os.path.join(self.settings["download_folder"], file)):
                mod_time = os.path.getmtime(os.path.join(self.settings["download_folder"], file))
                files.append((file, mod_time))
        
        # Sort by modification time, newest first
        files.sort(key=lambda x: x[1], reverse=True)
        return [file for file, _ in files] 