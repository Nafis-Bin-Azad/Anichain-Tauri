"use client";

import { useEffect, useState } from "react";
import Image from "next/image";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { PlayCircle, Clock, Star, ArrowLeft } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface AnimeDetails {
  title: string;
  original_title?: string;
  season_count: number;
  episode_count: number;
  special_count: number;
  year?: number;
  studio?: string;
  genres: string[];
  summary?: string;
  rating?: number;
  image_url?: string;
  episodes: Array<{
    number: number;
    file_name: string;
    path: string;
    is_special: boolean;
  }>;
}

export default function AnimePage({ params }: { params: { id: string } }) {
  const [anime, setAnime] = useState<AnimeDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchAnimeData = async () => {
      try {
        console.log("Anime page mounted with params:", params);
        // Decode the title from the URL parameter
        const decodedTitle = decodeURIComponent(params.id);
        console.log("Decoded title:", decodedTitle);

        // Get the download folder path from qBittorrent config
        const downloadFolder = await invoke<string>("get_download_folder");
        console.log("Download folder path:", downloadFolder);

        // Fetch metadata from the Rust backend
        console.log("Fetching metadata for:", decodedTitle);
        const metadata = await invoke("fetch_anime_metadata_command", {
          folderPath: `${downloadFolder}/${decodedTitle}`,
        });
        console.log("Received metadata:", metadata);

        if (metadata && Array.isArray(metadata) && metadata.length > 0) {
          const animeData = metadata[0];
          console.log("Setting anime data:", animeData);
          setAnime({
            title: animeData.title,
            original_title: animeData.original_title,
            season_count: animeData.season_count,
            episode_count: animeData.episode_count,
            special_count: animeData.special_count,
            year: animeData.year,
            studio: animeData.studio,
            genres: animeData.genres,
            summary: animeData.summary,
            rating: animeData.rating,
            image_url: animeData.image_url,
            episodes: animeData.episodes,
          });
          setError(null);
        } else {
          console.log("No metadata found or invalid response:", metadata);
          setError("No metadata found for this anime");
        }
      } catch (error) {
        console.error("Error fetching anime data:", error);
        setError(
          error instanceof Error ? error.message : "Failed to fetch anime data"
        );
      } finally {
        setLoading(false);
      }
    };

    fetchAnimeData();
  }, [params.id]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-screen">
        <div className="text-red-500 mb-4">{error}</div>
        <Button variant="ghost" onClick={() => window.history.back()}>
          <ArrowLeft className="w-4 h-4 mr-2" />
          Back to Downloads
        </Button>
      </div>
    );
  }

  if (!anime) {
    return (
      <div className="flex flex-col items-center justify-center h-screen">
        <div className="text-gray-500 mb-4">No anime data found</div>
        <Button variant="ghost" onClick={() => window.history.back()}>
          <ArrowLeft className="w-4 h-4 mr-2" />
          Back to Downloads
        </Button>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <Button
        variant="ghost"
        className="mb-6 hover:bg-transparent"
        onClick={() => window.history.back()}
      >
        <ArrowLeft className="w-4 h-4 mr-2" />
        Back to Downloads
      </Button>

      <div className="flex flex-col md:flex-row gap-8">
        {/* Left side - Image */}
        <div className="w-full md:w-1/3 lg:w-1/4">
          <div className="relative aspect-[3/4] rounded-lg overflow-hidden">
            <Image
              src={anime.image_url || "/placeholder.png"}
              alt={anime.title}
              fill
              className="object-cover"
            />
          </div>
        </div>

        {/* Right side - Info */}
        <div className="flex-1">
          <div className="space-y-4">
            <div>
              <h1 className="text-3xl font-bold">{anime.title}</h1>
              {anime.original_title && (
                <p className="text-gray-500">{anime.original_title}</p>
              )}
            </div>

            <div className="flex gap-4 items-center">
              {anime.year && (
                <Badge variant="secondary">
                  <Clock className="w-4 h-4 mr-1" />
                  {anime.year}
                </Badge>
              )}
              {anime.rating && (
                <Badge variant="secondary">
                  <Star className="w-4 h-4 mr-1" />
                  {anime.rating.toFixed(1)}
                </Badge>
              )}
              {anime.studio && (
                <Badge variant="secondary">{anime.studio}</Badge>
              )}
            </div>

            <div className="flex flex-wrap gap-2">
              {anime.genres.map((genre) => (
                <Badge key={genre} variant="outline">
                  {genre}
                </Badge>
              ))}
            </div>

            {anime.summary && (
              <p className="text-gray-600 dark:text-gray-300 leading-relaxed">
                {anime.summary}
              </p>
            )}

            <Button className="mt-4">
              <PlayCircle className="w-4 h-4 mr-2" />
              Play
            </Button>
          </div>
        </div>
      </div>

      {/* Seasons Section */}
      <div className="mt-12">
        <h2 className="text-2xl font-bold mb-6">Seasons</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {/* Specials Card */}
          {anime.special_count > 0 && (
            <Card className="hover:shadow-lg transition-shadow">
              <CardContent className="p-4">
                <div className="relative aspect-video rounded-md overflow-hidden mb-4">
                  <Image
                    src={anime.image_url || "/placeholder.png"}
                    alt="Specials"
                    fill
                    className="object-cover"
                  />
                  <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
                    <h3 className="text-white text-xl font-bold">Specials</h3>
                  </div>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-500">
                    {anime.special_count} Episodes
                  </span>
                  <Button variant="ghost" size="sm">
                    <PlayCircle className="w-4 h-4 mr-2" />
                    Play
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Regular Seasons */}
          {Array.from({ length: anime.season_count }).map((_, index) => (
            <Card key={index} className="hover:shadow-lg transition-shadow">
              <CardContent className="p-4">
                <div className="relative aspect-video rounded-md overflow-hidden mb-4">
                  <Image
                    src={anime.image_url || "/placeholder.png"}
                    alt={`Season ${index + 1}`}
                    fill
                    className="object-cover"
                  />
                  <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
                    <h3 className="text-white text-xl font-bold">
                      Season {index + 1}
                    </h3>
                  </div>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-500">
                    {anime.episode_count} Episodes
                  </span>
                  <Button variant="ghost" size="sm">
                    <PlayCircle className="w-4 h-4 mr-2" />
                    Play
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
