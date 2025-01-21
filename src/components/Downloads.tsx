"use client";

import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";
import DownloadCard from "./DownloadCard";
import { listen } from "@tauri-apps/api/event";

interface AnimeEpisode {
  number: number;
  file_name: string;
  path: string;
  is_special: boolean;
}

interface HamaMetadata {
  title: string;
  season_count: number;
  episode_count: number;
  special_count: number;
  year?: number;
  studio?: string;
  genres: string[];
  summary?: string;
  rating?: number;
  image_url?: string;
  episodes: AnimeEpisode[];
  specials: AnimeEpisode[];
}

export default function Downloads() {
  const [folders, setFolders] = useState<HamaMetadata[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState<number>(0);
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    // Load folders when component mounts or becomes visible
    if (isVisible) {
      loadFolders();
    }

    // Listen for download folder changes
    const unsubscribe = listen("download-folder-changed", () => {
      // Only reload if it's been more than 5 seconds since last update
      const now = Date.now();
      if (now - lastUpdate > 5000) {
        loadFolders();
        setLastUpdate(now);
      }
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, [isVisible, lastUpdate]);

  // Use Intersection Observer to detect when component is visible
  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsVisible(entry.isIntersecting);
      },
      { threshold: 0.1 }
    );

    const element = document.getElementById("downloads-container");
    if (element) {
      observer.observe(element);
    }

    return () => {
      if (element) {
        observer.unobserve(element);
      }
    };
  }, []);

  async function loadFolders() {
    try {
      setIsLoading(true);
      console.log("Fetching downloaded anime folders...");
      const scannedFolders = await invoke<HamaMetadata[]>(
        "scan_downloaded_anime"
      );

      console.log("Received folders:", scannedFolders);
      console.log(
        "Folders with images:",
        scannedFolders.map((folder) => ({
          title: folder.title,
          imageUrl: folder.image_url,
          episodes: folder.episodes.length,
          specials: folder.specials.length,
        }))
      );

      setFolders(scannedFolders);
    } catch (error) {
      console.error("Error loading folders:", error);
    } finally {
      setIsLoading(false);
    }
  }

  if (isLoading) {
    return (
      <div
        id="downloads-container"
        className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-7 gap-4 p-4"
      >
        {[...Array(12)].map((_, i) => (
          <div key={i} className="animate-pulse">
            <div className="bg-gray-300 rounded-lg aspect-[2/3]" />
          </div>
        ))}
      </div>
    );
  }

  if (folders.length === 0) {
    return (
      <div
        id="downloads-container"
        className="flex items-center justify-center h-full text-gray-500"
      >
        No anime found in downloads folder
      </div>
    );
  }

  return (
    <div
      id="downloads-container"
      className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-7 gap-4 p-4"
    >
      {folders.map((folder, index) => {
        console.log(`Rendering folder ${index}:`, {
          title: folder.title,
          imageUrl: folder.image_url,
          episodes: folder.episodes.length,
          specials: folder.specials.length,
        });

        return (
          <DownloadCard
            key={`${folder.title}-${index}`}
            title={folder.title}
            totalEpisodes={folder.episodes.length + folder.specials.length}
            seasonCount={folder.season_count}
            imageUrl={folder.image_url}
            onClick={() => {}}
            onDelete={async () => {
              await invoke("delete_downloaded_file", {
                filename: folder.episodes[0]?.path,
              });
              loadFolders();
            }}
          />
        );
      })}
    </div>
  );
}
