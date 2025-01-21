"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Card from "./Card";

interface DownloadedFile {
  filename: string;
  size: string;
}

export default function Downloads() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [files, setFiles] = useState<DownloadedFile[]>([]);
  const [downloadFolder, setDownloadFolder] = useState<string>("");

  useEffect(() => {
    loadFiles();
    loadDownloadFolder();
  }, []);

  const loadFiles = async () => {
    try {
      const downloadedFiles = await invoke<DownloadedFile[]>(
        "get_downloaded_files"
      );
      setFiles(downloadedFiles);
      setIsLoading(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load files");
      setIsLoading(false);
    }
  };

  const loadDownloadFolder = async () => {
    try {
      const folder = await invoke<string>("get_download_folder");
      setDownloadFolder(folder);
    } catch (err) {
      console.error("Failed to load download folder:", err);
    }
  };

  const handleDelete = async (filename: string) => {
    try {
      await invoke("delete_downloaded_file", { filename });
      // Refresh the file list
      loadFiles();
    } catch (err) {
      console.error("Failed to delete file:", err);
    }
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4 p-4">
        {[...Array(10)].map((_, i) => (
          <Card
            key={i}
            type="download"
            title=""
            episodeInfo=""
            isLoading={true}
          />
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  return (
    <div className="p-4">
      <div className="mb-4">
        <h2 className="text-lg font-semibold text-text-primary">
          Download Folder: {downloadFolder}
        </h2>
      </div>

      {files.length === 0 ? (
        <div className="flex items-center justify-center h-64">
          <p className="text-text-secondary">No downloaded files found</p>
        </div>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {files.map((file) => {
            // Extract series name and episode info from filename
            const parts = file.filename.split(" - ");
            const seriesName = parts[0].replace("[SubsPlease]", "").trim();
            const episodeInfo =
              parts[1]?.split("[")[0].trim() || "Unknown Episode";

            return (
              <Card
                key={file.filename}
                type="download"
                title={seriesName}
                episodeInfo={episodeInfo}
                fileSize={file.size}
                onDelete={() => handleDelete(file.filename)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
