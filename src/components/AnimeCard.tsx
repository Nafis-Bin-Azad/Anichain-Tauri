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
  onTrackChange: (isTracked: boolean) => Promise<void>;
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

  const handleTrack = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setLoading(true);
    try {
      await onTrackChange(isTracked);
    } catch (error) {
      console.error("Failed to track anime:", error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <div
        className="group relative bg-white rounded-lg shadow-md overflow-hidden cursor-pointer hover:shadow-lg transition-shadow duration-200"
        onClick={() => setShowDetails(true)}
      >
        <div className="aspect-[3/4] relative bg-gray-200">
          {imageUrl ? (
            <Image
              src={imageUrl}
              alt={title}
              fill
              className="object-cover"
              sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
            />
          ) : (
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="text-gray-400">No Image</span>
            </div>
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
