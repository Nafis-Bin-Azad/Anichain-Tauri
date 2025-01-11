"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface Settings {
  downloadFolder: string;
  rssUrl: string;
  qbHost: string;
  qbUsername: string;
  qbPassword: string;
}

export default function Settings() {
  const [settings, setSettings] = useState<Settings>({
    downloadFolder: "",
    rssUrl: "",
    qbHost: "",
    qbUsername: "",
    qbPassword: "",
  });
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const currentSettings = await invoke<Settings>("get_settings");
      setSettings(currentSettings);
    } catch (error) {
      console.error("Failed to load settings:", error);
      setMessage({
        type: "error",
        text: "Failed to load settings",
      });
    }
  };

  const handleBrowse = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: settings.downloadFolder,
      });
      if (selected) {
        setSettings((prev) => ({
          ...prev,
          downloadFolder: selected as string,
        }));
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await invoke("save_settings", { settings });
      setMessage({
        type: "success",
        text: "Settings saved successfully",
      });
    } catch (error) {
      console.error("Failed to save settings:", error);
      setMessage({
        type: "error",
        text: "Failed to save settings",
      });
    }
    setIsSaving(false);
  };

  return (
    <div className="bg-surface rounded-lg border border-gray-200 p-6 max-w-2xl mx-auto">
      <h2 className="text-xl font-bold text-text-primary mb-6">Settings</h2>

      {message && (
        <div
          className={`mb-4 p-4 rounded ${
            message.type === "success"
              ? "bg-green-50 text-success"
              : "bg-red-50 text-danger"
          }`}
        >
          {message.text}
        </div>
      )}

      <div className="space-y-6">
        {/* Download Folder */}
        <div>
          <label className="block text-text-primary font-medium mb-2">
            Download Folder
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={settings.downloadFolder}
              onChange={(e) =>
                setSettings((prev) => ({
                  ...prev,
                  downloadFolder: e.target.value,
                }))
              }
              className="flex-grow px-4 py-2 rounded-md border border-gray-200 focus:border-primary focus:ring-1 focus:ring-primary outline-none"
            />
            <button
              onClick={handleBrowse}
              className="px-4 py-2 bg-primary text-white rounded-md hover:bg-blue-600 transition-colors"
            >
              Browse
            </button>
          </div>
        </div>

        {/* RSS URL */}
        <div>
          <label className="block text-text-primary font-medium mb-2">
            RSS URL
          </label>
          <input
            type="text"
            value={settings.rssUrl}
            onChange={(e) =>
              setSettings((prev) => ({ ...prev, rssUrl: e.target.value }))
            }
            className="w-full px-4 py-2 rounded-md border border-gray-200 focus:border-primary focus:ring-1 focus:ring-primary outline-none"
          />
        </div>

        {/* qBittorrent Settings */}
        <div className="border-t border-gray-200 pt-6">
          <h3 className="text-lg font-bold text-text-primary mb-4">
            qBittorrent Settings
          </h3>

          <div className="space-y-4">
            <div>
              <label className="block text-text-primary font-medium mb-2">
                Host
              </label>
              <input
                type="text"
                value={settings.qbHost}
                onChange={(e) =>
                  setSettings((prev) => ({ ...prev, qbHost: e.target.value }))
                }
                className="w-full px-4 py-2 rounded-md border border-gray-200 focus:border-primary focus:ring-1 focus:ring-primary outline-none"
              />
            </div>

            <div>
              <label className="block text-text-primary font-medium mb-2">
                Username
              </label>
              <input
                type="text"
                value={settings.qbUsername}
                onChange={(e) =>
                  setSettings((prev) => ({
                    ...prev,
                    qbUsername: e.target.value,
                  }))
                }
                className="w-full px-4 py-2 rounded-md border border-gray-200 focus:border-primary focus:ring-1 focus:ring-primary outline-none"
              />
            </div>

            <div>
              <label className="block text-text-primary font-medium mb-2">
                Password
              </label>
              <input
                type="password"
                value={settings.qbPassword}
                onChange={(e) =>
                  setSettings((prev) => ({
                    ...prev,
                    qbPassword: e.target.value,
                  }))
                }
                className="w-full px-4 py-2 rounded-md border border-gray-200 focus:border-primary focus:ring-1 focus:ring-primary outline-none"
              />
            </div>
          </div>
        </div>

        {/* Save Button */}
        <div className="flex justify-end">
          <button
            onClick={handleSave}
            disabled={isSaving}
            className={`px-6 py-2 rounded-md text-white font-medium ${
              isSaving
                ? "bg-gray-400 cursor-not-allowed"
                : "bg-primary hover:bg-blue-600"
            } transition-colors`}
          >
            {isSaving ? "Saving..." : "Save Settings"}
          </button>
        </div>
      </div>
    </div>
  );
}
