"use client";

import { useEffect, useState } from "react";
import { tauri } from "@/lib/tauri";
import { Loader2, Save } from "lucide-react";
import { useToast } from "@/contexts/ToastContext";

interface Settings {
  rss_feed_url: string;
  download_path: string;
  qbittorrent_url: string;
  qbittorrent_username: string;
  qbittorrent_password: string;
}

export default function Settings() {
  const [settings, setSettings] = useState<Settings>({
    rss_feed_url: "",
    download_path: "",
    qbittorrent_url: "",
    qbittorrent_username: "",
    qbittorrent_password: "",
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const { showToast } = useToast();

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const data = await tauri.invoke<Settings>("get_settings");
        setSettings(data);
      } catch (error) {
        console.error("Failed to load settings:", error);
        showToast("Failed to load settings", "error");
      } finally {
        setLoading(false);
      }
    };

    loadSettings();
  }, [showToast]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    try {
      await tauri.invoke("save_settings", { settings });
      showToast("Settings saved successfully", "success");
    } catch (error) {
      console.error("Failed to save settings:", error);
      showToast("Failed to save settings", "error");
    } finally {
      setSaving(false);
    }
  };

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
        <h1 className="text-2xl font-bold text-gray-900">Settings</h1>
      </div>
      <form onSubmit={handleSubmit} className="space-y-6">
        <div className="bg-white shadow-sm rounded-lg p-6 space-y-6">
          <div className="space-y-2">
            <label
              htmlFor="rss_feed_url"
              className="block text-sm font-medium text-gray-700"
            >
              RSS Feed URL
            </label>
            <input
              type="url"
              id="rss_feed_url"
              value={settings.rss_feed_url}
              onChange={(e) =>
                setSettings({ ...settings, rss_feed_url: e.target.value })
              }
              className="block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              required
            />
          </div>

          <div className="space-y-2">
            <label
              htmlFor="download_path"
              className="block text-sm font-medium text-gray-700"
            >
              Download Path
            </label>
            <input
              type="text"
              id="download_path"
              value={settings.download_path}
              onChange={(e) =>
                setSettings({ ...settings, download_path: e.target.value })
              }
              className="block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              required
            />
          </div>

          <div className="space-y-2">
            <label
              htmlFor="qbittorrent_url"
              className="block text-sm font-medium text-gray-700"
            >
              qBittorrent URL
            </label>
            <input
              type="url"
              id="qbittorrent_url"
              value={settings.qbittorrent_url}
              onChange={(e) =>
                setSettings({ ...settings, qbittorrent_url: e.target.value })
              }
              className="block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              required
            />
          </div>

          <div className="space-y-2">
            <label
              htmlFor="qbittorrent_username"
              className="block text-sm font-medium text-gray-700"
            >
              qBittorrent Username
            </label>
            <input
              type="text"
              id="qbittorrent_username"
              value={settings.qbittorrent_username}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  qbittorrent_username: e.target.value,
                })
              }
              className="block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              required
            />
          </div>

          <div className="space-y-2">
            <label
              htmlFor="qbittorrent_password"
              className="block text-sm font-medium text-gray-700"
            >
              qBittorrent Password
            </label>
            <input
              type="password"
              id="qbittorrent_password"
              value={settings.qbittorrent_password}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  qbittorrent_password: e.target.value,
                })
              }
              className="block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              required
            />
          </div>
        </div>

        <div className="flex justify-end">
          <button
            type="submit"
            disabled={saving}
            className="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {saving ? (
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            ) : (
              <Save className="w-4 h-4 mr-2" />
            )}
            Save Settings
          </button>
        </div>
      </form>
    </div>
  );
}
