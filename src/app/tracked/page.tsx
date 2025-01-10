"use client";

import { useEffect, useState } from "react";
import { invokeTauri } from "@/lib/tauri";
import { Loader2 } from "lucide-react";
import AnimeCard from "@/components/AnimeCard";

interface TrackedAnime {
  title: string;
  episode: string;
  image_path: string | null;
}

export default function Tracked() {
  const [trackedAnime, setTrackedAnime] = useState<TrackedAnime[]>([]);
  const [loading, setLoading] = useState(true);

  const loadTrackedAnime = async () => {
    try {
      const data = await invokeTauri<TrackedAnime[]>(
        "get_tracked_anime_details"
      );
      setTrackedAnime(data);
    } catch (error) {
      console.error("Failed to load tracked anime:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTrackedAnime();
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
        <h1 className="text-2xl font-bold text-gray-900">Tracked Anime</h1>
        <p className="text-sm text-gray-500">
          {trackedAnime.length} series tracked
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-6">
        {trackedAnime.map((anime) => (
          <AnimeCard
            key={anime.title}
            title={anime.title}
            episode={anime.episode}
            isTracked={true}
            onTrackChange={loadTrackedAnime}
          />
        ))}
      </div>
    </div>
  );
}
