import React from "react";
import { Badge } from "@/components/ui/badge";

interface AnimeDetailsProps {
  title: string;
  year?: number;
  rating?: number;
  duration?: string;
  genres?: string[];
  summary?: string;
  studio?: string;
  imageUrl?: string;
  episodeCount: number;
  specialCount: number;
  onClose: () => void;
}

export default function AnimeDetails({
  title,
  year,
  rating,
  duration,
  genres = [],
  summary,
  studio,
  imageUrl,
  episodeCount,
  specialCount,
  onClose,
}: AnimeDetailsProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="relative w-full max-w-4xl bg-background p-6 rounded-lg shadow-xl">
        <button
          onClick={onClose}
          className="absolute top-4 right-4 text-muted-foreground hover:text-foreground"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
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

        <div className="flex gap-6">
          <div className="w-1/3">
            <img
              src={imageUrl || "/placeholder.png"}
              alt={title}
              className="w-full rounded-lg object-cover aspect-[3/4]"
            />
          </div>

          <div className="w-2/3">
            <div className="flex items-baseline gap-4">
              <h2 className="text-2xl font-bold">{title}</h2>
              {year && <span className="text-muted-foreground">{year}</span>}
            </div>

            <div className="mt-4 flex items-center gap-4">
              {rating && (
                <div className="flex items-center gap-1">
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                    className="text-yellow-500"
                  >
                    <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
                  </svg>
                  <span>{rating.toFixed(1)}</span>
                </div>
              )}
              {duration && <span>{duration}</span>}
            </div>

            {genres.length > 0 && (
              <div className="mt-4 flex flex-wrap gap-2">
                {genres.map((genre) => (
                  <Badge key={genre} variant="secondary">
                    {genre}
                  </Badge>
                ))}
              </div>
            )}

            {summary && <p className="mt-4 text-muted-foreground">{summary}</p>}

            {studio && (
              <div className="mt-4">
                <span className="font-semibold">Studio:</span> {studio}
              </div>
            )}

            <div className="mt-4">
              <div className="flex gap-4">
                <div>
                  <span className="font-semibold">Episodes:</span>{" "}
                  {episodeCount}
                </div>
                {specialCount > 0 && (
                  <div>
                    <span className="font-semibold">Specials:</span>{" "}
                    {specialCount}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
