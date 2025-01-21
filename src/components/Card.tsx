"use client";

import { useState } from "react";

interface CardSkeletonProps {
  className?: string;
}

export function CardSkeleton({}: CardSkeletonProps) {
  return (
    <div className="aspect-[2/3] w-full">
      <div className="relative h-full rounded-xl overflow-hidden bg-gray-100 animate-pulse">
        {/* Image placeholder */}
        <div className="w-full h-full bg-gray-200" />

        {/* Title and info placeholder */}
        <div className="absolute bottom-0 left-0 right-0 bg-black/70 p-4">
          <div className="h-4 bg-gray-300 rounded w-3/4 mb-2"></div>
          <div className="h-3 bg-gray-400 rounded w-1/2"></div>
        </div>
      </div>
    </div>
  );
}

interface CardProps {
  type: "available" | "tracked" | "download";
  title: string;
  episodeInfo: string;
  imageUrl?: string | null;
  isTracked?: boolean;
  onTrackToggle?: (magnetUrl?: string) => void;
  onUntrack?: () => void;
  onDelete?: () => void;
  isLoading?: boolean;
  magnetUrl?: string;
  fileSize?: string;
}

export default function Card({
  type,
  title,
  episodeInfo,
  imageUrl,
  isTracked,
  onTrackToggle,
  onUntrack,
  onDelete,
  isLoading = false,
  magnetUrl,
  fileSize,
}: CardProps) {
  const [isFlipped, setIsFlipped] = useState(false);

  // Show loading skeleton if loading or if title is empty (initial state)
  const showSkeleton = isLoading || !title;

  return (
    <div
      className={`relative w-full aspect-[2/3] rounded-xl overflow-hidden transform-gpu transition-transform duration-500 ease-in-out ${
        isFlipped ? "rotate-y-180" : ""
      }`}
      onClick={() => !showSkeleton && setIsFlipped(!isFlipped)}
    >
      {/* Front of card */}
      <div
        className={`absolute inset-0 w-full h-full bg-gray-800 transform-gpu transition-opacity duration-500 ${
          isFlipped ? "opacity-0" : "opacity-100"
        }`}
      >
        {showSkeleton ? (
          // Loading skeleton
          <div className="w-full h-full animate-pulse">
            <div className="w-full h-full bg-gray-700" />
            <div className="absolute bottom-0 left-0 right-0 p-4 bg-black bg-opacity-70">
              <div className="h-4 bg-gray-700 rounded w-3/4 mb-2" />
              <div className="h-4 bg-gray-700 rounded w-1/2" />
            </div>
          </div>
        ) : (
          // Actual content
          <>
            {imageUrl ? (
              <img
                src={imageUrl}
                alt={title}
                className="w-full h-full object-cover"
                loading="lazy"
              />
            ) : (
              <div className="w-full h-full bg-gray-700 flex items-center justify-center">
                <span className="text-gray-400">No image available</span>
              </div>
            )}
            <div className="absolute bottom-0 left-0 right-0 p-4 bg-black bg-opacity-70">
              <h3 className="text-white font-semibold mb-1 line-clamp-2">
                {title}
              </h3>
              <p className="text-gray-300 text-sm">{episodeInfo}</p>
              {fileSize && (
                <p className="text-gray-300 text-sm mt-1">Size: {fileSize}</p>
              )}
            </div>
          </>
        )}
      </div>

      {/* Back of card */}
      <div
        className={`absolute inset-0 w-full h-full bg-white transform-gpu transition-opacity duration-500 rotate-y-180 ${
          isFlipped ? "opacity-100" : "opacity-0"
        }`}
      >
        <div className="p-4">
          <h3 className="font-semibold mb-2">{title}</h3>
          {type === "available" && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onTrackToggle?.(magnetUrl);
              }}
              className={`mt-4 px-4 py-2 rounded-lg w-full ${
                isTracked
                  ? "bg-red-500 hover:bg-red-600"
                  : "bg-blue-500 hover:bg-blue-600"
              } text-white transition-colors`}
            >
              {isTracked ? "Untrack" : "Track"}
            </button>
          )}
          {type === "tracked" && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onUntrack?.();
              }}
              className="mt-4 px-4 py-2 rounded-lg w-full bg-red-500 hover:bg-red-600 text-white transition-colors"
            >
              Untrack
            </button>
          )}
          {type === "download" && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDelete?.();
              }}
              className="mt-4 px-4 py-2 rounded-lg w-full bg-red-500 hover:bg-red-600 text-white transition-colors"
            >
              Delete
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
