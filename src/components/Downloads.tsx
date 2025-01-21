"use client";

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import DownloadCard from "./DownloadCard";

interface AnimeEpisode {
  number: number;
  file_name: string;
  path: string;
  is_special: boolean;
}

interface AnimeMetadata {
  title: string;
  season_count: number;
  episodes: AnimeEpisode[];
  specials: AnimeEpisode[];
  episode_count: number;
  special_count: number;
  image_url?: string;
}

interface DownloadsProps {
  onAnimeSelect: (title: string) => void;
}

export default function Downloads({ onAnimeSelect }: DownloadsProps) {
  const [animeList, setAnimeList] = useState<AnimeMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadCachedData = async () => {
    try {
      const cachedData = await invoke<AnimeMetadata[]>(
        "get_cached_anime_metadata"
      );
      console.log("Received cached data:", cachedData);
      setAnimeList(cachedData);
    } catch (e) {
      console.error("Error loading cached data:", e);
      setError("Failed to load cached data");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCachedData();

    // Listen for updates from the background task
    const unlisten = listen<void>("anime_data_ready", () => {
      console.log("Received anime_data_ready event, reloading cached data");
      loadCachedData();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-red-500">
        {error}
      </div>
    );
  }

  if (!animeList || animeList.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        No anime found
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10 gap-4 p-4">
      {animeList.map((anime) => (
        <DownloadCard
          key={anime.title}
          title={anime.title}
          totalEpisodes={anime.episode_count + anime.special_count}
          seasonCount={anime.season_count}
          imageUrl={anime.image_url}
          onDelete={() => {
            // TODO: Implement delete functionality
          }}
          onClick={() => {
            onAnimeSelect(anime.title);
          }}
        />
      ))}
    </div>
  );
}
