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

const AnimePage = () => {
  const [animeData, setAnimeData] = useState<AnimeData>({
    animeList: [],
    trackedAnime: [],
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isTauri, setIsTauri] = useState(false);

  const fetchAnimeData = async () => {
    try {
      const [rssFeed, trackedAnime] = await Promise.all([
        invokeTauri<AnimeEntry[]>("fetch_rss_feed"),
        invokeTauri<string[]>("get_tracked_anime"),
      ]);

      return {
        animeList: rssFeed,
        trackedAnime,
      };
    } catch (err) {
      console.error("Error fetching anime data:", err);
      throw new Error("Failed to fetch anime data");
    }
  };

  useEffect(() => {
    const checkTauri = async () => {
      try {
        await invokeTauri("ping");
        setIsTauri(true);
      } catch (err) {
        console.log("Not running in Tauri context");
        setIsTauri(false);
      }
    };

    const loadData = async () => {
      try {
        const data = await fetchAnimeData();
        setAnimeData(data);
        setError(null);
      } catch (err) {
        setError("Failed to load anime data. Please try again.");
      } finally {
        setLoading(false);
      }
    };

    checkTauri();
    loadData();
  }, []);

  if (loading) {
    console.log("Rendering loading state...");
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  if (error) {
    console.log("Rendering error state:", error);
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] space-y-4">
        <p className="text-lg font-medium text-red-600">{error}</p>
        <button
          onClick={() => {
            console.log("Retry button clicked, reloading page...");
            window.location.reload();
          }}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600"
        >
          Try Again
        </button>
      </div>
    );
  }

  const handleTrackChange = async () => {
    console.log("Track change triggered, refreshing anime data...");
    try {
      const data = await fetchAnimeData();
      console.log("Anime data refreshed after track change:", {
        totalAnime: data.animeList.length,
        trackedCount: data.trackedAnime.length,
      });
      setAnimeData(data);
    } catch (err) {
      console.error("Failed to refresh anime data:", err);
    }
  };

  console.log("Rendering main content...");
  return (
    <div className="space-y-8">
      {!isTauri && (
        <div className="bg-yellow-50 border-l-4 border-yellow-400 p-4">
          <div className="flex">
            <div className="ml-3">
              <p className="text-sm text-yellow-700">
                Running in development mode with mock data. For real data, run
                with{" "}
                <code className="bg-yellow-100 px-1 py-0.5 rounded">
                  npm run tauri dev
                </code>
              </p>
            </div>
          </div>
        </div>
      )}
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-gray-900">Latest Anime</h1>
          <p className="text-sm text-gray-500">
            {animeData.animeList.length} series available
          </p>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-6">
          {animeData.animeList.map((anime) => (
            <AnimeCard
              key={anime.title}
              title={anime.title}
              episode={`Released: ${anime.date}`}
              isTracked={animeData.trackedAnime.includes(anime.title)}
              onTrackChange={handleTrackChange}
              imageUrl={anime.image_url}
              summary={anime.summary}
            />
          ))}
        </div>
      </div>

      {animeData.trackedAnime.length > 0 && (
        <div className="space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-bold text-gray-900">Tracked Anime</h2>
            <p className="text-sm text-gray-500">
              {animeData.trackedAnime.length} series tracked
            </p>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-6">
            {animeData.trackedAnime.map((title) => {
              const anime = animeData.animeList.find((a) => a.title === title);
              if (!anime) return null;
              return (
                <AnimeCard
                  key={title}
                  title={title}
                  episode={`Released: ${anime.date}`}
                  isTracked={true}
                  onTrackChange={handleTrackChange}
                  imageUrl={anime.image_url}
                  summary={anime.summary}
                />
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default AnimePage;
