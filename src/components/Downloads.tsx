"use client";

import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback, useMemo, useRef } from "react";
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
  const lastUpdateRef = useRef<number>(Date.now());

  const loadFolders = useCallback(async () => {
    try {
      setIsLoading(true);
      const scannedFolders = await invoke<HamaMetadata[]>(
        "scan_downloaded_anime"
      );
      const uniqueFolders = Array.from(
        new Map(scannedFolders.map((folder) => [folder.title, folder])).values()
      );

      // Log essential information about loaded anime
      console.log(
        "Loaded anime:",
        uniqueFolders.map((folder) => ({
          title: folder.title,
          totalEpisodes: folder.episodes.length + folder.specials.length,
          imageUrl: folder.image_url,
        }))
      );

      setFolders(uniqueFolders);
    } catch (error) {
      console.error("Error loading folders:", error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleFolderChange = useCallback(() => {
    const now = Date.now();
    if (now - lastUpdateRef.current > 5000) {
      loadFolders();
      lastUpdateRef.current = now;
    }
  }, [loadFolders]);

  useEffect(() => {
    loadFolders();

    // Listen for download folder changes
    const unsubscribe = listen("download-folder-changed", handleFolderChange);

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, [handleFolderChange]);

  const handleDelete = useCallback(
    async (filename: string) => {
      await invoke("delete_downloaded_file", { filename });
      loadFolders();
    },
    [loadFolders]
  );

  const renderContent = useMemo(() => {
    if (isLoading) {
      return (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-7 gap-4 p-4">
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
        <div className="flex items-center justify-center h-full text-gray-500">
          No anime found in downloads folder
        </div>
      );
    }

    return (
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-7 gap-4 p-4">
        {folders.map((folder) => (
          <DownloadCard
            key={folder.title}
            title={folder.title}
            totalEpisodes={folder.episodes.length + folder.specials.length}
            seasonCount={folder.season_count}
            imageUrl={folder.image_url}
            onClick={() => {}}
            onDelete={() => handleDelete(folder.episodes[0]?.path)}
          />
        ))}
      </div>
    );
  }, [folders, isLoading, handleDelete]);

  return <div id="downloads-container">{renderContent}</div>;
}
