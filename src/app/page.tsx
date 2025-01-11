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

  useEffect(() => {
    loadAnimeData();
  }, []);

  const loadAnimeData = async () => {
    try {
      // Create 12 placeholder cards immediately
      const placeholders = Array(12)
        .fill(null)
        .map((_, index) => ({
          metadata: {
            title: "",
            image_url: "",
            synopsis: "",
            score: null,
            episodes: null,
            status: "Loading...",
            season: null,
            year: null,
          },
          latest_episode: {
            title: "",
            number: 0,
            magnet_url: "",
            size: "",
            release_date: "",
          },
        }));
      setAvailableAnime(placeholders);

      // Start loading real data
      await invoke("refresh_anime_list");

      // Poll for updates every second
      const interval = setInterval(async () => {
        const animeList = await invoke<AnimeInfo[]>("get_available_anime");
        setAvailableAnime(animeList);

        // Stop polling if no more loading items
        if (
          !animeList.some((anime) => anime.metadata.status === "Loading...")
        ) {
          clearInterval(interval);
        }
      }, 1000);

      // Cleanup interval on component unmount
      return () => clearInterval(interval);
    } catch (error) {
      console.error("Failed to load anime:", error);
      setAvailableAnime([]);
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
    switch (activeTab) {
      case "available":
        return (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {availableAnime.map((anime, index) => (
              <Card
                key={
                  anime.metadata.title
                    ? `${anime.metadata.title}-${anime.latest_episode.number}`
                    : `placeholder-${index}`
                }
                type="available"
                title={anime.metadata.title}
                episodeInfo={
                  anime.latest_episode.number
                    ? `Episode ${anime.latest_episode.number}`
                    : ""
                }
                imageUrl={anime.metadata.image_url || null}
                isTracked={trackedAnime.has(anime.metadata.title)}
                onTrackToggle={() => handleTrackToggle(anime.metadata.title)}
                isLoading={
                  !anime.metadata.title ||
                  anime.metadata.status === "Loading..."
                }
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
                  key={`${anime.metadata.title}-${anime.latest_episode.number}`}
                  type="tracked"
                  title={anime.metadata.title}
                  episodeInfo={`Episode ${anime.latest_episode.number}`}
                  imageUrl={anime.metadata.image_url || null}
                  onUntrack={() => handleUntrack(anime.metadata.title)}
                  isLoading={anime.metadata.status === "Loading..."}
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
