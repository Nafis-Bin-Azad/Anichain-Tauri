"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import Image from "next/image";

interface AnimeMetadata {
  title: string;
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

  useEffect(() => {
    const fetchAnimeData = async () => {
      try {
        console.log("Fetching anime data...");
        const metadata = await invoke("fetch_anime_metadata_command", {
          folderPath: "/Users/nafislord/Downloads/Anime",
        });
        console.log("Received metadata:", metadata);

        if (metadata && Array.isArray(metadata)) {
          setAnimeList(metadata);
          console.log("Updated anime list:", metadata);
        }
      } catch (error) {
        console.error("Error fetching anime data:", error);
      } finally {
        setLoading(false);
      }
    };

    fetchAnimeData();
  }, []);

  const handleAnimeClick = (title: string) => {
    console.log("Card clicked for anime:", title);
    onAnimeSelect(title);
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold mb-8">Downloads</h1>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
        {animeList.map((anime, index) => (
          <div
            key={index}
            onClick={() => handleAnimeClick(anime.title)}
            className="cursor-pointer"
          >
            <Card className="hover:shadow-lg transition-shadow h-full">
              <CardContent className="p-4">
                <div className="relative aspect-[3/4] rounded-md overflow-hidden mb-4">
                  <Image
                    src={anime.image_url || "/placeholder.png"}
                    alt={anime.title}
                    fill
                    className="object-cover"
                  />
                </div>
                <h3 className="font-semibold mb-2 line-clamp-2">
                  {anime.title}
                </h3>
                <div className="text-sm text-gray-500">
                  {anime.episode_count} Episodes
                  {anime.special_count > 0 &&
                    ` • ${anime.special_count} Specials`}
                </div>
              </CardContent>
            </Card>
          </div>
        ))}
      </div>
    </div>
  );
}
