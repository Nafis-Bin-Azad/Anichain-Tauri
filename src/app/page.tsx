"use client";

import { useState, useEffect } from "react";
import Layout from "@/components/Layout";
import AnimeCard from "@/components/AnimeCard";
import TrackedAnimeCard from "@/components/TrackedAnimeCard";
import DownloadCard from "@/components/DownloadCard";
import Schedule from "@/components/Schedule";
import Settings from "@/components/Settings";
import { invoke } from "@tauri-apps/api/core";

interface AnimeEntry {
  title: string;
  imageUrl: string;
  episodeInfo: string;
}

export default function Home() {
  const [activeTab, setActiveTab] = useState("available");
  const [animeList, setAnimeList] = useState<AnimeEntry[]>([]);
  const [trackedAnime, setTrackedAnime] = useState<AnimeEntry[]>([]);
  const [downloads, setDownloads] = useState<AnimeEntry[]>([]);

  useEffect(() => {
    loadInitialData();
  }, []);

  const loadInitialData = async () => {
    try {
      // Load available anime
      const availableAnime = await invoke<AnimeEntry[]>("get_available_anime");
      setAnimeList(availableAnime);

      // Load tracked anime
      const tracked = await invoke<AnimeEntry[]>("get_tracked_anime");
      setTrackedAnime(tracked);

      // Load downloads
      const downloadsList = await invoke<AnimeEntry[]>("get_downloads");
      setDownloads(downloadsList);
    } catch (error) {
      console.error("Failed to load initial data:", error);
    }
  };

  const handleTrackToggle = async (title: string) => {
    try {
      const isTracked = trackedAnime.some((anime) => anime.title === title);
      if (isTracked) {
        await invoke("untrack_anime", { title });
        setTrackedAnime((prev) =>
          prev.filter((anime) => anime.title !== title)
        );
      } else {
        await invoke("track_anime", { title });
        const anime = animeList.find((a) => a.title === title);
        if (anime) {
          setTrackedAnime((prev) => [...prev, anime]);
        }
      }
    } catch (error) {
      console.error("Failed to toggle tracking:", error);
    }
  };

  const handleDeleteDownload = async (filename: string) => {
    try {
      await invoke("delete_download", { filename });
      setDownloads((prev) =>
        prev.filter((download) => download.title !== filename)
      );
    } catch (error) {
      console.error("Failed to delete download:", error);
    }
  };

  const renderContent = () => {
    switch (activeTab) {
      case "available":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {animeList.map((anime) => (
              <AnimeCard
                key={anime.title}
                title={anime.title}
                imageUrl={anime.imageUrl}
                episodeInfo={anime.episodeInfo}
                isTracked={trackedAnime.some((a) => a.title === anime.title)}
                onTrackToggle={() => handleTrackToggle(anime.title)}
              />
            ))}
          </div>
        );

      case "tracked":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {trackedAnime.map((anime) => (
              <TrackedAnimeCard
                key={anime.title}
                seriesName={anime.title}
                imageUrl={anime.imageUrl}
                onUntrack={() => handleTrackToggle(anime.title)}
              />
            ))}
          </div>
        );

      case "downloads":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {downloads.map((download) => (
              <DownloadCard
                key={download.title}
                filename={download.title}
                imageUrl={download.imageUrl}
                onDelete={() => handleDeleteDownload(download.title)}
              />
            ))}
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
      <div className="min-h-screen bg-background">{renderContent()}</div>
    </Layout>
  );
}
