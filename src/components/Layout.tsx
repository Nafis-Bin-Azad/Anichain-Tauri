"use client";

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface LayoutProps {
  children: React.ReactNode;
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

interface ConnectionStatus {
  is_connected: boolean;
  error_message: string | null;
}

export default function Layout({
  children,
  activeTab,
  setActiveTab,
}: LayoutProps) {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({
    is_connected: false,
    error_message: null,
  });

  useEffect(() => {
    // Listen for connection status updates
    const unlistenStatus = listen<ConnectionStatus>(
      "qbittorrent-status",
      (event) => {
        setConnectionStatus(event.payload);
      }
    );

    // Listen for tab switch requests
    const unlistenSwitch = listen("switch-to-settings", () => {
      setActiveTab("settings");
    });

    // Check initial connection status
    invoke("check_qbittorrent_connection");

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenSwitch.then((fn) => fn());
    };
  }, []);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Navigation */}
      <nav className="bg-white shadow-sm">
        <div className="container mx-auto px-4">
          <div className="flex space-x-8 h-16">
            <button
              onClick={() => setActiveTab("available")}
              className={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
                activeTab === "available"
                  ? "border-blue-500 text-gray-900"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Available
            </button>
            <button
              onClick={() => setActiveTab("tracked")}
              className={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
                activeTab === "tracked"
                  ? "border-blue-500 text-gray-900"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Tracked
            </button>
            <button
              onClick={() => setActiveTab("downloads")}
              className={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
                activeTab === "downloads"
                  ? "border-blue-500 text-gray-900"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Downloads
            </button>
            <button
              onClick={() => setActiveTab("schedule")}
              className={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
                activeTab === "schedule"
                  ? "border-blue-500 text-gray-900"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Schedule
            </button>
            <button
              onClick={() => setActiveTab("settings")}
              className={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
                activeTab === "settings"
                  ? "border-blue-500 text-gray-900"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Settings
            </button>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-8">{children}</main>

      {/* Status Bar */}
      <div className="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200 px-3 py-2">
        <div className="container mx-auto flex items-center space-x-2">
          <span
            className={`text-lg ${
              connectionStatus.is_connected ? "text-green-500" : "text-red-500"
            }`}
          >
            ●
          </span>
          <span
            className={`text-sm font-medium ${
              connectionStatus.is_connected ? "text-green-500" : "text-red-500"
            }`}
          >
            {connectionStatus.is_connected
              ? "qBittorrent Connected"
              : "qBittorrent Disconnected"}
          </span>
          {connectionStatus.error_message && (
            <span className="text-sm text-red-500 ml-2">
              ({connectionStatus.error_message})
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
