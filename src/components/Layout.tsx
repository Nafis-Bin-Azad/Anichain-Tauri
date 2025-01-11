"use client";

import { useState } from "react";

interface LayoutProps {
  children: React.ReactNode;
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

export default function Layout({
  children,
  activeTab,
  setActiveTab,
}: LayoutProps) {
  const [searchQuery, setSearchQuery] = useState("");

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: Implement search functionality
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Navigation Bar */}
      <nav className="bg-white border-b border-gray-200">
        <div className="container mx-auto px-5 py-3">
          <div className="flex items-center space-x-8">
            {/* Logo */}
            <h1 className="text-2xl font-bold text-text-primary">ANICHAIN</h1>

            {/* Navigation Buttons */}
            <div className="flex space-x-4">
              {[
                "Available",
                "Schedule",
                "Tracked",
                "Downloads",
                "Settings",
              ].map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab.toLowerCase())}
                  className={`px-4 py-2 text-sm font-medium transition-colors ${
                    activeTab === tab.toLowerCase()
                      ? "text-primary font-bold"
                      : "text-gray-600 hover:text-gray-900"
                  }`}
                >
                  {tab}
                </button>
              ))}
            </div>

            {/* Search Bar */}
            <div className="flex-grow">
              <form onSubmit={handleSearch} className="flex items-center">
                <div className="flex-grow relative">
                  <div className="flex items-center bg-gray-50 rounded-l-full rounded-r-full">
                    <input
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="Search anime"
                      className="w-full py-2 px-4 bg-transparent border-none focus:outline-none text-sm"
                    />
                    <button
                      type="submit"
                      className="ml-2 px-6 py-2 bg-primary text-white rounded-full text-sm font-medium hover:bg-blue-600 transition-colors"
                    >
                      Search
                    </button>
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <div className="container mx-auto px-5 py-6">{children}</div>

      {/* Status Bar */}
      <div className="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200 px-5 py-2">
        <div className="container mx-auto flex items-center space-x-2">
          <span className="text-green-500 text-lg">●</span>
          <span className="text-sm font-medium text-green-500">
            qBittorrent Connected
          </span>
        </div>
      </div>
    </div>
  );
}
