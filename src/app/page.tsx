"use client";

import { useState } from "react";
import Layout from "@/components/Layout";
import Card from "@/components/Card";
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
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {mockAnimeList.map((anime) => (
              <Card
                key={anime.title}
                type="available"
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
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {mockAnimeList
              .filter((anime) => trackedAnime.has(anime.title))
              .map((anime) => (
                <Card
                  key={anime.title}
                  type="tracked"
                  title={anime.title}
                  episodeInfo={anime.episodeInfo}
                  imageUrl={anime.imageUrl}
                  onUntrack={() => handleUntrack(anime.title)}
                />
              ))}
          </div>
        );

      case "downloads":
        return (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4 auto-rows-fr">
            {downloads.map((download) => (
              <Card
                key={download.filename}
                type="download"
                title={download.filename}
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
