"use client";

import { useEffect, useState } from "react";
import { invokeTauri } from "@/lib/tauri";
import {
  Loader2,
  Download,
  Trash2,
  CheckCircle,
  AlertCircle,
} from "lucide-react";

interface Download {
  title: string;
  episode: string;
  progress: number;
  status: "downloading" | "completed" | "failed";
  error?: string;
}

export default function Downloads() {
  const [downloads, setDownloads] = useState<Download[]>([]);
  const [loading, setLoading] = useState(true);

  const loadDownloads = async () => {
    try {
      const data = await invokeTauri<Download[]>("get_downloads");
      setDownloads(data);
    } catch (error) {
      console.error("Failed to load downloads:", error);
    } finally {
      setLoading(false);
    }
  };

  const removeDownload = async (title: string, episode: string) => {
    try {
      await invokeTauri("remove_download", { title, episode });
      loadDownloads();
    } catch (error) {
      console.error("Failed to remove download:", error);
    }
  };

  useEffect(() => {
    loadDownloads();
    const interval = setInterval(loadDownloads, 1000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[calc(100vh-8rem)]">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Downloads</h1>
        <p className="text-sm text-gray-500">
          {downloads.length} downloads in progress
        </p>
      </div>
      <div className="space-y-4">
        {downloads.map((download) => (
          <div
            key={`${download.title}-${download.episode}`}
            className="bg-white rounded-lg shadow-sm p-4"
          >
            <div className="flex items-center justify-between mb-2">
              <div>
                <h3 className="text-lg font-semibold text-gray-900">
                  {download.title}
                </h3>
                <p className="text-sm text-gray-600">
                  Episode {download.episode}
                </p>
              </div>
              <div className="flex items-center space-x-2">
                {download.status === "downloading" && (
                  <Download className="w-5 h-5 text-blue-500 animate-bounce" />
                )}
                {download.status === "completed" && (
                  <CheckCircle className="w-5 h-5 text-green-500" />
                )}
                {download.status === "failed" && (
                  <AlertCircle className="w-5 h-5 text-red-500" />
                )}
                <button
                  onClick={() =>
                    removeDownload(download.title, download.episode)
                  }
                  className="p-1 hover:bg-gray-100 rounded-full transition-colors"
                >
                  <Trash2 className="w-5 h-5 text-gray-500" />
                </button>
              </div>
            </div>
            {download.status === "downloading" && (
              <div className="relative pt-1">
                <div className="overflow-hidden h-2 text-xs flex rounded bg-blue-100">
                  <div
                    style={{ width: `${download.progress}%` }}
                    className="shadow-none flex flex-col text-center whitespace-nowrap text-white justify-center bg-blue-500 transition-all duration-300"
                  />
                </div>
                <div className="text-right mt-1">
                  <span className="text-sm font-semibold text-blue-600">
                    {download.progress}%
                  </span>
                </div>
              </div>
            )}
            {download.status === "failed" && download.error && (
              <p className="mt-2 text-sm text-red-600">{download.error}</p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
