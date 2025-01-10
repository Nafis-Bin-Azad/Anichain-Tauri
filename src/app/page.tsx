"use client";

import { useEffect, useState } from "react";
import { invokeTauri } from "@/lib/tauri";
import { Loader2 } from "lucide-react";
import AnimeCard from "@/components/AnimeCard";

interface AnimeEntry {
  title: string;
  link: string;
  date: string;
  image_url?: string;
  summary?: string;
}

interface AnimeData {
  animeList: AnimeEntry[];
  trackedAnime: string[];
}

export default function AnimePage() {
  const [animeData, setAnimeData] = useState<AnimeData>({
    animeList: [],
    trackedAnime: [],
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadData = async () => {
      try {
        console.log("Fetching anime data...");
        const [rssFeed, trackedAnime] = await Promise.all([
          invokeTauri<AnimeEntry[]>("fetch_rss_feed"),
          invokeTauri<string[]>("get_tracked_anime"),
        ]);

        console.log("RSS Feed data:", rssFeed);
        console.log("Tracked anime:", trackedAnime);

        setAnimeData({
          animeList: rssFeed,
          trackedAnime,
        });
        setError(null);
      } catch (err) {
        console.error("Error loading data:", err);
        setError("Failed to load anime data. Please try again.");
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, []);

  const handleTrackChange = async (title: string, isTracking: boolean) => {
    try {
      if (isTracking) {
        await invokeTauri("untrack_anime", { title });
      } else {
        await invokeTauri("track_anime", { title });
      }

      // Refresh tracked anime list
      const trackedAnime = await invokeTauri<string[]>("get_tracked_anime");
      setAnimeData((prev) => ({
        ...prev,
        trackedAnime,
      }));
    } catch (err) {
      console.error("Failed to update tracking:", err);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] space-y-4">
        <p className="text-lg font-medium text-red-600">{error}</p>
        <button
          onClick={() => window.location.reload()}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600"
        >
          Try Again
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Available Anime</h1>
        <p className="text-sm text-gray-500">
          {animeData.animeList.length} series available
        </p>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-6">
        {animeData.animeList.map((anime) => (
          <AnimeCard
            key={anime.title}
            title={anime.title}
            episode={anime.date}
            isTracked={animeData.trackedAnime.includes(anime.title)}
            onTrackChange={(isTracked) =>
              handleTrackChange(anime.title, isTracked)
            }
            imageUrl={anime.image_url}
            summary={anime.summary}
          />
        ))}
      </div>
    </div>
  );
}
