"use client";

import { useState } from "react";
import Layout from "@/components/Layout";
import AnimeCard from "@/components/AnimeCard";
import TrackedAnimeCard from "@/components/TrackedAnimeCard";
import DownloadCard from "@/components/DownloadCard";
import Schedule from "@/components/Schedule";
import Settings from "@/components/Settings";
import { mockAnimeList, mockDownloads } from "@/lib/mockData";

export default function Home() {
  const [activeTab, setActiveTab] = useState("available");
  const [trackedAnime, setTrackedAnime] = useState<Set<string>>(new Set());
  const [downloads, setDownloads] = useState(mockDownloads);

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

  const handleDeleteDownload = (filename: string) => {
    setDownloads((prev) => prev.filter((d) => d.filename !== filename));
  };

  const renderContent = () => {
    switch (activeTab) {
      case "available":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {mockAnimeList.map((anime) => (
              <AnimeCard
                key={anime.title}
                title={anime.title}
                episodeInfo={anime.episodeInfo}
                imageUrl={anime.imageUrl}
                isTracked={trackedAnime.has(anime.title)}
                onTrackToggle={() => handleTrackToggle(anime.title)}
              />
            ))}
          </div>
        );

      case "tracked":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {mockAnimeList
              .filter((anime) => trackedAnime.has(anime.title))
              .map((anime) => (
                <TrackedAnimeCard
                  key={anime.title}
                  seriesName={anime.title}
                  imageUrl={anime.imageUrl}
                  onUntrack={() => handleUntrack(anime.title)}
                />
              ))}
          </div>
        );

      case "downloads":
        return (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
            {downloads.map((download) => (
              <DownloadCard
                key={download.filename}
                filename={download.filename}
                imageUrl={download.imageUrl}
                onDelete={() => handleDeleteDownload(download.filename)}
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
