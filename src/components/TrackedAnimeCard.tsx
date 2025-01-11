"use client";

import { useState, useEffect } from "react";

interface TrackedAnimeCardProps {
  seriesName: string;
  imageUrl: string;
  onUntrack: () => void;
}

interface AnimeStatus {
  status: "Ongoing" | "Ended";
  lastEpisode: string;
  nextEpisodeTime?: Date;
}

export default function TrackedAnimeCard({
  seriesName,
  imageUrl,
  onUntrack,
}: TrackedAnimeCardProps) {
  const [isFlipped, setIsFlipped] = useState(false);
  const [status] = useState<AnimeStatus>({
    status: "Ongoing",
    lastEpisode: "None",
  });
  const [countdown, setCountdown] = useState<string>("");

  useEffect(() => {
    if (status.nextEpisodeTime) {
      const timer = setInterval(() => {
        const now = new Date();
        const diff = status.nextEpisodeTime!.getTime() - now.getTime();

        if (diff <= 0) {
          setCountdown("Episode available now!");
          clearInterval(timer);
        } else {
          const days = Math.floor(diff / (1000 * 60 * 60 * 24));
          const hours = Math.floor(
            (diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60)
          );
          const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

          let countdownText = "Next episode in: ";
          if (days > 0) countdownText += `${days}d `;
          countdownText += `${hours}h ${minutes}m`;

          setCountdown(countdownText);
        }
      }, 60000); // Update every minute

      return () => clearInterval(timer);
    }
  }, [status.nextEpisodeTime]);

  return (
    <div
      className={`card w-[220px] h-[380px] cursor-pointer relative transform transition-transform duration-500 ${
        status.status === "Ended" ? "border-danger" : ""
      }`}
      onClick={() => setIsFlipped(!isFlipped)}
    >
      {/* Front of card */}
      <div className={`absolute w-full h-full ${isFlipped ? "hidden" : ""}`}>
        <div className="flex flex-col items-center space-y-4">
          <div className="relative w-[200px] h-[280px]">
            <img
              src={imageUrl || "/placeholder.png"}
              alt={seriesName}
              className="w-[200px] h-[280px] object-cover rounded-lg"
            />
          </div>
          <h3 className="text-text-primary font-bold text-center line-clamp-2">
            {seriesName}
          </h3>
          <div className="text-sm font-medium">
            {status.status === "Ended" ? (
              <span className="text-danger">Series Ended ✓</span>
            ) : (
              <span className="text-secondary">✓ Tracking</span>
            )}
          </div>
          {status.status === "Ended" && (
            <p className="text-danger text-xs text-center bg-red-50 p-2 rounded">
              Series has finished airing.
              <br />
              Click to remove from tracking.
            </p>
          )}
        </div>
      </div>

      {/* Back of card */}
      <div className={`absolute w-full h-full ${!isFlipped ? "hidden" : ""}`}>
        <div className="flex flex-col h-full">
          <h3 className="text-text-primary font-bold mb-2 line-clamp-2">
            {seriesName}
          </h3>
          <p className="text-text-secondary text-sm mb-2">
            Last episode: {status.lastEpisode}
          </p>
          <div className="bg-green-50 p-2 rounded mb-4">
            <p className="text-secondary text-sm font-medium">{countdown}</p>
          </div>
          <p className="text-sm mb-4">
            Status:{" "}
            <span
              className={
                status.status === "Ended"
                  ? "text-danger font-bold"
                  : "text-secondary font-bold"
              }
            >
              {status.status}
            </span>
          </p>
          <div className="flex-grow" />
          <button
            onClick={(e) => {
              e.stopPropagation();
              onUntrack();
            }}
            className="w-full py-2 px-4 rounded-md bg-danger text-white text-sm font-medium hover:bg-red-600 transition-colors"
          >
            Stop Tracking
          </button>
        </div>
      </div>
    </div>
  );
}
