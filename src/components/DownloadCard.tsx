"use client";

import React, { useState, useEffect } from "react";

interface DownloadCardProps {
  title: string;
  totalEpisodes: number;
  seasonCount: number;
  imageUrl?: string;
  onDelete: () => void;
  onClick: () => void;
}

export default function DownloadCard({
  title,
  totalEpisodes,
  seasonCount,
  imageUrl,
  onDelete,
  onClick,
}: DownloadCardProps) {
  const [currentImageUrl, setCurrentImageUrl] = useState<string>(
    imageUrl || "/placeholder.jpg"
  );
  const [isImageLoading, setIsImageLoading] = useState(true);

  // Update currentImageUrl when imageUrl prop changes
  useEffect(() => {
    if (imageUrl) {
      setCurrentImageUrl(imageUrl);
    }
  }, [imageUrl]);

  console.log(`Rendering DownloadCard for ${title}:`, {
    imageUrl: currentImageUrl,
    totalEpisodes,
    seasonCount,
  });

  const handleImageError = (
    e: React.SyntheticEvent<HTMLImageElement, Event>
  ) => {
    console.warn(`Failed to load image for ${title}:`, e);
    const img = e.currentTarget;

    // Try alternative image formats if the first one fails
    if (img.src.endsWith(".jpg")) {
      setCurrentImageUrl(img.src.replace(".jpg", ".png"));
    } else if (img.src.endsWith(".png")) {
      setCurrentImageUrl(img.src.replace(".png", ".webp"));
    } else if (img.src.endsWith(".webp")) {
      setCurrentImageUrl("/placeholder.jpg");
    }
  };

  const handleImageLoad = () => {
    setIsImageLoading(false);
  };

  return (
    <div className="group flex flex-col cursor-pointer" onClick={onClick}>
      <div className="relative aspect-[3/4] rounded-lg overflow-hidden bg-gray-100">
        {isImageLoading && (
          <div className="absolute inset-0 bg-gray-200 animate-pulse" />
        )}
        <img
          src={currentImageUrl}
          alt={title}
          className={`w-full h-full object-cover transition-transform group-hover:scale-105 ${
            isImageLoading ? "opacity-0" : "opacity-100"
          }`}
          loading="lazy"
          onError={handleImageError}
          onLoad={handleImageLoad}
        />

        <div className="absolute top-2 right-2">
          <div className="px-2 py-1 rounded bg-amber-500 text-black font-bold text-sm">
            {totalEpisodes}
          </div>
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="absolute top-2 left-2 p-2 rounded-full bg-black/50 text-white opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/70"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div className="mt-2 px-1">
        <h3 className="font-medium text-sm line-clamp-2">{title}</h3>
        <p className="text-xs text-gray-500 mt-1">
          {seasonCount} season{seasonCount !== 1 ? "s" : ""}
        </p>
      </div>
    </div>
  );
}
