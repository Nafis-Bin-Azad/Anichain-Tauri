"use client";

import { useState, useEffect } from "react";
import Layout from "@/components/Layout";
import Card from "@/components/Card";
import Schedule from "@/components/Schedule";
import Settings from "@/components/Settings";
import { invoke } from "@tauri-apps/api/core";

interface AnimeMetadata {
  title: string;
  image_url: string;
  synopsis: string;
  score: number | null;
  episodes: number | null;
  status: string;
  season: string | null;
  year: number | null;
}

interface Episode {
  title: string;
  number: number;
  magnet_url: string;
  size: string;
  release_date: string;
}

interface AnimeInfo {
  metadata: AnimeMetadata;
  latest_episode: Episode;
}

export default function Home() {
  const [activeTab, setActiveTab] = useState("available");
  const [trackedAnime, setTrackedAnime] = useState<Set<string>>(new Set());
  const [availableAnime, setAvailableAnime] = useState<AnimeInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadAnimeData();
  }, []);

  const loadAnimeData = async () => {
    try {
      setIsLoading(true);
      const animeList = await invoke<AnimeInfo[]>("get_available_anime");
      setAvailableAnime(animeList);
    } catch (error) {
      console.error("Failed to load anime:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleTrackToggle = (title: string) => {
    setTrackedAnime((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(title)) {
        newSet.delete(title);
      } else {
        newSet.add(title);
      }
      return newSet;
    });
  };

  const handleUntrack = (title: string) => {
    setTrackedAnime((prev) => {
      const newSet = new Set(prev);
      newSet.delete(title);
      return newSet;
    });
  };

  const renderContent = () => {
    if (isLoading) {
      return (
        <div className="flex justify-center items-center h-64">
          <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500"></div>
        </div>
      );
    }

    switch (activeTab) {
      case "available":
        return (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {availableAnime.map((anime) => (
              <Card
                key={anime.metadata.title}
                type="available"
                title={anime.metadata.title}
                episodeInfo={`Episode ${anime.latest_episode.number}`}
                imageUrl={anime.metadata.image_url}
                isTracked={trackedAnime.has(anime.metadata.title)}
                onTrackToggle={() => handleTrackToggle(anime.metadata.title)}
              />
            ))}
          </div>
        );

      case "tracked":
        return (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {availableAnime
              .filter((anime) => trackedAnime.has(anime.metadata.title))
              .map((anime) => (
                <Card
                  key={anime.metadata.title}
                  type="tracked"
                  title={anime.metadata.title}
                  episodeInfo={`Episode ${anime.latest_episode.number}`}
                  imageUrl={anime.metadata.image_url}
                  onUntrack={() => handleUntrack(anime.metadata.title)}
                />
              ))}
          </div>
        );

      case "downloads":
        return (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            <div className="col-span-full text-center text-gray-500">
              Downloads will be shown here
            </div>
          </div>
        );

      case "schedule":
        return <Schedule />;

      case "settings":
        return <Settings />;

      default:
        return null;
    }
  };

  return (
    <Layout activeTab={activeTab} setActiveTab={setActiveTab}>
      {renderContent()}
    </Layout>
  );
}
