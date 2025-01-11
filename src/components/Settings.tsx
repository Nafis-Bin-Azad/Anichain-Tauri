"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface QBittorrentConfig {
  url: string;
  username: string;
  password: string;
}

interface Settings {
  qbittorrent?: QBittorrentConfig;
}

export default function Settings() {
  const [isLoading, setIsLoading] = useState(true);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [config, setConfig] = useState<QBittorrentConfig>({
    url: "",
    username: "",
    password: "",
  });

  useEffect(() => {
    loadSettings();
    checkConnection();
  }, []);

  const loadSettings = async () => {
    try {
      const settings: Settings = await invoke("get_settings");
      if (settings.qbittorrent) {
        setConfig(settings.qbittorrent);
      }
      setIsLoading(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load settings");
      setIsLoading(false);
    }
  };

  const checkConnection = async () => {
    try {
      const connected = await invoke("check_qbittorrent_connection");
      setIsConnected(!!connected);
    } catch (err) {
      console.error("Connection check failed:", err);
      setIsConnected(false);
    }
  };

  const handleConnect = async () => {
    setIsConnecting(true);
    setError(null);
    try {
      await invoke("save_qbittorrent_settings", { config });
      setIsConnected(true);
    } catch (err) {
      setError(err as string);
      setIsConnected(false);
    } finally {
      setIsConnecting(false);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setConfig((prev) => ({ ...prev, [name]: value }));
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold mb-6 text-text-primary">Settings</h2>

      <div className="bg-background-secondary rounded-lg p-6 shadow-lg">
        <h3 className="text-xl font-semibold mb-4 text-text-primary">
          qBittorrent Connection
        </h3>

        <div className="space-y-4">
          <div>
            <label className="block text-text-primary font-medium mb-2">
              Host
            </label>
            <input
              type="text"
              name="url"
              value={config.url}
              onChange={handleInputChange}
              placeholder="http://localhost:8080"
              className="w-full px-4 py-2 rounded-lg bg-background-primary border border-border focus:outline-none focus:ring-2 focus:ring-primary text-text-primary"
            />
          </div>

          <div>
            <label className="block text-text-primary font-medium mb-2">
              Username
            </label>
            <input
              type="text"
              name="username"
              value={config.username}
              onChange={handleInputChange}
              className="w-full px-4 py-2 rounded-lg bg-background-primary border border-border focus:outline-none focus:ring-2 focus:ring-primary text-text-primary"
            />
          </div>

          <div>
            <label className="block text-text-primary font-medium mb-2">
              Password
            </label>
            <input
              type="password"
              name="password"
              value={config.password}
              onChange={handleInputChange}
              className="w-full px-4 py-2 rounded-lg bg-background-primary border border-border focus:outline-none focus:ring-2 focus:ring-primary text-text-primary"
            />
          </div>

          {error && <div className="text-red-500 text-sm mt-2">{error}</div>}

          <div className="flex items-center justify-between mt-6">
            <button
              onClick={handleConnect}
              disabled={isConnecting}
              className="px-6 py-2 bg-primary text-white rounded-lg hover:bg-primary-dark focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-2"
            >
              {isConnecting ? (
                <>
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  <span>Connecting...</span>
                </>
              ) : (
                <span>Connect</span>
              )}
            </button>

            <div className="flex items-center space-x-2">
              <div
                className={`w-3 h-3 rounded-full ${
                  isConnected ? "bg-green-500" : "bg-red-500"
                }`}
              ></div>
              <span className="text-text-secondary">
                {isConnected ? "Connected" : "Not Connected"}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
