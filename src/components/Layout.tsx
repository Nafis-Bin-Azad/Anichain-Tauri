"use client";

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LayoutProps {
  children: React.ReactNode;
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

const navItems = [
  { id: "available", label: "Available" },
  { id: "schedule", label: "Schedule" },
  { id: "tracked", label: "Tracked" },
  { id: "downloads", label: "Downloads" },
  { id: "settings", label: "Settings" },
];

export default function Layout({
  children,
  activeTab,
  setActiveTab,
}: LayoutProps) {
  const [searchQuery, setSearchQuery] = useState("");

  const handleSearch = async () => {
    try {
      // TODO: Implement search functionality using Tauri
      await invoke("search_anime", { query: searchQuery });
    } catch (error) {
      console.error("Search failed:", error);
    }
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Navigation Bar */}
      <nav className="bg-surface border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            {/* Logo */}
            <div className="text-2xl font-bold text-text-primary">ANICHAIN</div>

            {/* Navigation Buttons */}
            <div className="hidden md:flex space-x-4">
              {navItems.map((item) => (
                <button
                  key={item.id}
                  onClick={() => setActiveTab(item.id)}
                  className={`nav-button ${
                    activeTab === item.id ? "nav-button-active" : ""
                  }`}
                >
                  {item.label}
                </button>
              ))}
            </div>

            {/* Search Bar */}
            <div className="flex items-center bg-background rounded-full px-4 py-2">
              <input
                type="text"
                placeholder="Search anime"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="bg-transparent border-none outline-none text-text-primary placeholder-text-secondary"
                onKeyPress={(e) => e.key === "Enter" && handleSearch()}
              />
              <button
                onClick={handleSearch}
                className="ml-2 px-4 py-2 bg-primary text-white rounded-full text-sm hover:bg-blue-600 transition-colors"
              >
                Search
              </button>
            </div>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {children}
      </main>

      {/* Status Bar */}
      <div className="fixed bottom-0 left-0 right-0 h-8 bg-surface border-t border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-full flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <span className="text-secondary">●</span>
            <span className="text-sm text-text-secondary">
              qBittorrent Connected
            </span>
          </div>
          <button className="text-sm text-primary hover:text-blue-600 transition-colors">
            Reconnect
          </button>
        </div>
      </div>
    </div>
  );
}
