"use client";

import { useEffect, useState } from "react";
import { invokeTauri } from "@/lib/tauri";
import { Loader2, Star, Calendar, Film } from "lucide-react";
import Image from "next/image";
import { useToast } from "@/contexts/ToastContext";

interface AnimeDetails {
  title: string;
  synopsis: string;
  image_url: string;
  episodes: number | null;
  status: string;
  score: number | null;
  aired: string | null;
  genres: string[];
}

interface AnimeDetailsProps {
  title: string;
  onClose: () => void;
}

export default function AnimeDetails({ title, onClose }: AnimeDetailsProps) {
  const [details, setDetails] = useState<AnimeDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const { showToast } = useToast();

  useEffect(() => {
    const fetchDetails = async () => {
      try {
        const data = await invokeTauri<AnimeDetails>("fetch_anime_details", {
          title,
        });
        setDetails(data);

        // Cache the image
        if (data.image_url) {
          await invokeTauri("cache_anime_image", {
            url: data.image_url,
            title: data.title,
          });
        }
      } catch (error) {
        console.error("Failed to fetch anime details:", error);
        showToast("Failed to fetch anime details", "error");
      } finally {
        setLoading(false);
      }
    };

    fetchDetails();
  }, [title, showToast]);

  if (loading) {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center">
        <div className="bg-white p-6 rounded-lg shadow-xl">
          <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
        </div>
      </div>
    );
  }

  if (!details) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 animate-fade-in">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden animate-slide-up">
        <div className="relative h-64 md:h-96">
          <Image
            src={details.image_url}
            alt={details.title}
            fill
            className="object-cover"
          />
          <button
            onClick={onClose}
            className="absolute top-4 right-4 p-2 bg-black/50 rounded-full text-white hover:bg-black/70 transition-colors"
          >
            ×
          </button>
        </div>
        <div className="p-6 space-y-4">
          <h2 className="text-2xl font-bold text-gray-900">{details.title}</h2>

          <div className="flex flex-wrap gap-4 text-sm text-gray-600">
            {details.score && (
              <div className="flex items-center">
                <Star className="w-4 h-4 mr-1 text-yellow-400" />
                {details.score.toFixed(1)}
              </div>
            )}
            {details.aired && (
              <div className="flex items-center">
                <Calendar className="w-4 h-4 mr-1" />
                {details.aired}
              </div>
            )}
            {details.episodes && (
              <div className="flex items-center">
                <Film className="w-4 h-4 mr-1" />
                {details.episodes} episodes
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            {details.genres.map((genre) => (
              <span
                key={genre}
                className="px-2 py-1 bg-blue-50 text-blue-600 rounded-full text-sm"
              >
                {genre}
              </span>
            ))}
          </div>

          <p className="text-gray-600 leading-relaxed">{details.synopsis}</p>

          <div className="pt-4">
            <div className="inline-block px-3 py-1 rounded-full bg-gray-100 text-gray-700">
              {details.status}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
