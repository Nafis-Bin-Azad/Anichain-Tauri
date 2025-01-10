"use client";

import { useState } from "react";
import Image from "next/image";
import { invokeTauri } from "@/lib/tauri";
import { Loader2 } from "lucide-react";
import AnimeDetails from "./AnimeDetails";

interface AnimeCardProps {
  title: string;
  episode: string;
  isTracked: boolean;
  onTrackChange: () => void;
  imageUrl?: string;
  summary?: string;
}

export default function AnimeCard({
  title,
  episode,
  isTracked,
  onTrackChange,
  imageUrl,
  summary,
}: AnimeCardProps) {
  const [loading, setLoading] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const handleClick = () => {
    setShowDetails(true);
  };

  const handleTrack = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setLoading(true);
    try {
      if (isTracked) {
        await invokeTauri("untrack_anime", { title });
      } else {
        await invokeTauri("track_anime", { title });
      }
      onTrackChange();
    } catch (error) {
      console.error("Failed to update tracking:", error);
    }
    setLoading(false);
  };

  return (
    <>
      <div
        onClick={handleClick}
        className="group relative bg-white rounded-lg shadow-sm hover:shadow-md transition-shadow duration-200 overflow-hidden cursor-pointer"
      >
        <div className="aspect-[2/3] relative bg-gray-100">
          {imageUrl ? (
            <Image
              src={imageUrl}
              alt={title}
              fill
              className="object-cover transition-transform duration-200 group-hover:scale-105"
            />
          ) : (
            <div className="w-full h-full bg-gray-200 animate-pulse" />
          )}
        </div>
        <div className="p-4 space-y-2">
          <h3 className="text-sm font-semibold text-gray-900 line-clamp-2 group-hover:text-blue-600 transition-colors">
            {title}
          </h3>
          <p className="text-sm text-gray-600">{episode}</p>
          {summary && (
            <p className="text-xs text-gray-500 line-clamp-2">{summary}</p>
          )}
          <button
            onClick={handleTrack}
            disabled={loading}
            className={`w-full py-2 px-4 rounded-md text-sm font-medium transition-all duration-200 ${
              isTracked
                ? "bg-red-500 hover:bg-red-600 text-white"
                : "bg-blue-500 hover:bg-blue-600 text-white"
            } ${loading ? "opacity-75 cursor-not-allowed" : ""}`}
          >
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin mx-auto" />
            ) : isTracked ? (
              "Untrack"
            ) : (
              "Track"
            )}
          </button>
        </div>
      </div>

      {showDetails && (
        <AnimeDetails title={title} onClose={() => setShowDetails(false)} />
      )}
    </>
  );
}
