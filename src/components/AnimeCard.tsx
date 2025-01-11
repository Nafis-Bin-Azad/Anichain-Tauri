"use client";

import { useState } from "react";
import Image from "next/image";
import { invoke } from "@tauri-apps/api/core";

interface AnimeCardProps {
  title: string;
  episodeInfo: string;
  imageUrl: string;
  isTracked: boolean;
  onTrackToggle: () => void;
}

export default function AnimeCard({
  title,
  episodeInfo,
  imageUrl,
  isTracked,
  onTrackToggle,
}: AnimeCardProps) {
  const [isFlipped, setIsFlipped] = useState(false);
  const [description, setDescription] = useState<string>(
    "Loading description..."
  );

  const loadDescription = async () => {
    try {
      const desc = await invoke<string>("get_anime_description", { title });
      setDescription(desc);
    } catch (error) {
      setDescription("Failed to load description");
      console.error("Failed to load description:", error);
    }
  };

  const handleClick = () => {
    setIsFlipped(!isFlipped);
    if (!isFlipped) {
      loadDescription();
    }
  };

  const cleanTitle = title.replace("[SubsPlease]", "").split(" - ")[0].trim();

  return (
    <div
      className="card w-[220px] h-[380px] cursor-pointer relative transform transition-transform duration-500"
      onClick={handleClick}
    >
      {/* Front of card */}
      <div className={`absolute w-full h-full ${isFlipped ? "hidden" : ""}`}>
        <div className="flex flex-col items-center space-y-4">
          <div className="relative w-[200px] h-[280px]">
            <Image
              src={imageUrl || "/placeholder.png"}
              alt={cleanTitle}
              fill
              className="object-cover rounded-lg"
            />
          </div>
          <h3 className="text-text-primary font-bold text-center line-clamp-2">
            {cleanTitle}
          </h3>
          <p className="text-text-secondary text-sm">{episodeInfo}</p>
          <div className="text-sm font-medium">
            {isTracked ? (
              <span className="text-secondary">✓ Tracking</span>
            ) : (
              <span className="text-primary">Click to View Info</span>
            )}
          </div>
        </div>
      </div>

      {/* Back of card */}
      <div className={`absolute w-full h-full ${!isFlipped ? "hidden" : ""}`}>
        <div className="flex flex-col h-full">
          <h3 className="text-text-primary font-bold mb-2 line-clamp-2">
            {cleanTitle}
          </h3>
          <p className="text-text-secondary text-sm mb-4">
            Episode: {episodeInfo}
          </p>
          <div className="flex-grow overflow-y-auto mb-4">
            <p className="text-text-primary text-sm">{description}</p>
          </div>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onTrackToggle();
            }}
            className={`w-full py-2 px-4 rounded-md text-white text-sm font-medium transition-colors ${
              isTracked
                ? "bg-danger hover:bg-red-600"
                : "bg-primary hover:bg-blue-600"
            }`}
          >
            {isTracked ? "Untrack Series" : "Track Series"}
          </button>
        </div>
      </div>
    </div>
  );
}
