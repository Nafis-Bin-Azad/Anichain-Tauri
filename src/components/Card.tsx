"use client";

import { useState } from "react";

interface CardProps {
  title: string;
  imageUrl: string;
  type: "available" | "tracked" | "download";
  episodeInfo?: string;
  isTracked?: boolean;
  onTrackToggle?: () => void;
  onUntrack?: () => void;
  onDelete?: () => void;
}

const FALLBACK_IMAGE = "/placeholder.jpg";

export default function Card({
  title,
  imageUrl,
  type,
  episodeInfo,
  isTracked,
  onTrackToggle,
  onUntrack,
  onDelete,
}: CardProps) {
  const [isFlipped, setIsFlipped] = useState(false);
  const [imageError, setImageError] = useState(false);

  const handleImageError = () => {
    setImageError(true);
  };

  const handleFlip = () => {
    setIsFlipped(!isFlipped);
  };

  const handleMouseLeave = () => {
    if (isFlipped) {
      setIsFlipped(false);
    }
  };

  const handleBackClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains("card-back-container")) {
      handleFlip();
    }
  };

  const renderCardFront = () => (
    <div
      className="absolute inset-0 backface-hidden rounded-xl overflow-hidden cursor-pointer group/card"
      onClick={handleFlip}
    >
      <div className="relative h-full">
        <img
          src={imageError ? FALLBACK_IMAGE : imageUrl}
          alt={title}
          onError={handleImageError}
          className="w-full h-full object-cover"
        />

        <div className="absolute bottom-0 left-0 right-0 bg-black/70 group-hover/card:bg-black/90 transition-colors p-4">
          <h3 className="font-medium text-sm text-white line-clamp-2">
            {title}
          </h3>
          {episodeInfo && (
            <p className="text-xs text-gray-300 mt-1">{episodeInfo}</p>
          )}
        </div>
      </div>
    </div>
  );

  const renderCardBack = () => (
    <div
      className="absolute inset-0 backface-hidden rotate-y-180 rounded-xl overflow-hidden"
      onClick={handleBackClick}
    >
      <div className="card-back-container h-full bg-white/95 backdrop-blur-sm p-4 flex flex-col">
        <h3 className="font-medium mb-4 text-center">{title}</h3>

        {type === "tracked" && (
          <div className="flex-grow flex flex-col justify-center">
            <p className="text-sm text-gray-600 mb-4 text-center">
              Latest Episode: {episodeInfo}
            </p>
            <button
              onClick={onUntrack}
              className="w-full py-2 px-4 bg-danger text-white rounded-lg hover:bg-red-600 transition-colors"
            >
              Untrack
            </button>
          </div>
        )}

        {type === "download" && (
          <div className="flex-grow flex flex-col justify-center">
            <button
              onClick={onDelete}
              className="w-full py-2 px-4 bg-danger text-white rounded-lg hover:bg-red-600 transition-colors"
            >
              Delete
            </button>
          </div>
        )}

        {type === "available" && (
          <div className="flex-grow flex flex-col justify-center">
            <p className="text-sm text-gray-600 mb-4 text-center">
              {episodeInfo}
            </p>
            <button
              onClick={onTrackToggle}
              className={`w-full py-2 px-4 rounded-lg transition-colors ${
                isTracked
                  ? "bg-danger text-white hover:bg-red-600"
                  : "bg-primary text-white hover:bg-blue-600"
              }`}
            >
              {isTracked ? "Untrack" : "Track"}
            </button>
          </div>
        )}
      </div>
    </div>
  );

  return (
    <div className="aspect-[2/3] w-full">
      <div
        className="relative perspective group h-full"
        onMouseLeave={handleMouseLeave}
      >
        <div
          className={`relative w-full h-full preserve-3d transition-transform duration-500 shadow-lg rounded-xl ${
            isFlipped ? "rotate-y-180" : ""
          }`}
        >
          {renderCardFront()}
          {renderCardBack()}
        </div>
      </div>
    </div>
  );
}
