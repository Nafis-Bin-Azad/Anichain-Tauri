"use client";

import { useEffect, useState } from "react";
import { tauri } from "@/lib/tauri";
import AnimeCard from "@/components/AnimeCard";
import { Loader2 } from "lucide-react";

interface AnimeEntry {
  title: string;
  link: string;
  description: string;
  pub_date: string;
  image_path: string | null;
}

export default function Home() {
  const [animeList, setAnimeList] = useState<AnimeEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [trackedAnime, setTrackedAnime] = useState<string[]>([]);

  const loadData = async () => {
    try {
      const [feed, tracked] = await Promise.all([
        tauri.invoke<AnimeEntry[]>("fetch_rss_feed"),
        tauri.invoke<string[]>("get_tracked_anime"),
      ]);
      setAnimeList(feed);
      setTrackedAnime(tracked);
    } catch (error) {
      console.error("Failed to load data:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Available Anime</h1>
        <p className="text-sm text-gray-500">
          {animeList.length} series available
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-6">
        {animeList.map((anime) => (
          <AnimeCard
            key={anime.title}
            title={anime.title}
            episode={
              anime.title.split(" - ")[1]?.split("[")[0]?.trim() || "Unknown"
            }
            isTracked={trackedAnime.includes(anime.title)}
            onTrackChange={loadData}
          />
        ))}
      </div>
    </div>
  );
}
