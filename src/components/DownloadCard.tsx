"use client";

import { useState, useEffect } from "react";
import Image from "next/image";
import { invoke } from "@tauri-apps/api/core";

interface DownloadCardProps {
  filename: string;
  imageUrl: string;
  onDelete: () => void;
}

interface TorrentStatus {
  progress: number;
  isDownloading: boolean;
}

export default function DownloadCard({
  filename,
  imageUrl,
  onDelete,
}: DownloadCardProps) {
  const [isFlipped, setIsFlipped] = useState(false);
  const [status, setStatus] = useState<TorrentStatus>({
    progress: 0,
    isDownloading: false,
  });
  const [fileSize, setFileSize] = useState<string>("");

  useEffect(() => {
    loadFileInfo();
    const timer = setInterval(updateProgress, 1000);
    return () => clearInterval(timer);
  }, []);

  const loadFileInfo = async () => {
    try {
      const size = await invoke<number>("get_file_size", { filename });
      setFileSize(`${(size / (1024 * 1024)).toFixed(1)} MB`);
    } catch (error) {
      console.error("Failed to load file info:", error);
    }
  };

  const updateProgress = async () => {
    try {
      const torrentStatus = await invoke<TorrentStatus>("get_torrent_status", {
        filename,
      });
      setStatus(torrentStatus);
    } catch (error) {
      console.error("Failed to update progress:", error);
    }
  };

  const cleanTitle = filename
    .replace("[SubsPlease]", "")
    .split(" - ")[0]
    .trim();
  const episodeInfo = filename.split(" - ")[1]?.split("[")[0].trim() || "";

  return (
    <div
      className="card w-[220px] h-[380px] cursor-pointer relative transform transition-transform duration-500"
      onClick={() => setIsFlipped(!isFlipped)}
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
          <p className="text-text-secondary text-sm">Episode {episodeInfo}</p>
          {status.isDownloading && (
            <div className="w-full">
              <div className="progress-bar">
                <div
                  className="progress-bar-fill"
                  style={{ width: `${status.progress}%` }}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Back of card */}
      <div className={`absolute w-full h-full ${!isFlipped ? "hidden" : ""}`}>
        <div className="flex flex-col h-full">
          <h3 className="text-text-primary font-bold mb-2 line-clamp-2">
            {filename}
          </h3>
          <div className="text-text-secondary text-sm space-y-2 mb-4">
            <p>Size: {fileSize}</p>
            {status.isDownloading && (
              <p>Progress: {status.progress.toFixed(1)}%</p>
            )}
          </div>
          <div className="flex-grow" />
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            className="w-full py-2 px-4 rounded-md bg-danger text-white text-sm font-medium hover:bg-red-600 transition-colors"
          >
            Delete Episode
          </button>
        </div>
      </div>
    </div>
  );
}
