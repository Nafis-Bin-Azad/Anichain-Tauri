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
  const [isSearchExpanded, setIsSearchExpanded] = useState(false);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: Implement search functionality
    setIsSearchExpanded(false);
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Navigation Bar */}
      <nav className="bg-white border-b border-gray-200">
        <div className="container mx-auto px-3 py-3">
          <div className="flex items-center justify-between">
            {/* Logo */}
            <h1 className="text-2xl font-bold text-text-primary shrink-0">
              ANICHAIN
            </h1>

            {/* Navigation Buttons */}
            <div className="hidden md:flex space-x-1 lg:space-x-4 shrink-0">
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
                  className={`px-3 lg:px-4 py-2 text-sm font-medium transition-colors ${
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
            <div className="relative flex items-center ml-2">
              {isSearchExpanded ? (
                <form onSubmit={handleSearch} className="flex items-center">
                  <div className="flex items-center bg-gray-50 rounded-l-full rounded-r-full">
                    <input
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="Search anime"
                      className="w-full py-2 px-4 bg-transparent border-none focus:outline-none text-sm"
                      autoFocus
                    />
                    <button
                      type="submit"
                      className="ml-2 px-6 py-2 bg-primary text-white rounded-full text-sm font-medium hover:bg-blue-600 transition-colors"
                    >
                      Search
                    </button>
                  </div>
                </form>
              ) : (
                <button
                  onClick={() => setIsSearchExpanded(true)}
                  className="p-2 hover:bg-gray-100 rounded-full transition-colors"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    strokeWidth={1.5}
                    stroke="currentColor"
                    className="w-6 h-6"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
                    />
                  </svg>
                </button>
              )}
            </div>

            {/* Mobile Menu Button */}
            <button className="md:hidden p-2 hover:bg-gray-100 rounded-lg">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth={1.5}
                stroke="currentColor"
                className="w-6 h-6"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
                />
              </svg>
            </button>
          </div>

          {/* Mobile Navigation Menu */}
          <div className="md:hidden mt-2">
            {["Available", "Schedule", "Tracked", "Downloads", "Settings"].map(
              (tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab.toLowerCase())}
                  className={`block w-full text-left px-4 py-2 text-sm font-medium transition-colors ${
                    activeTab === tab.toLowerCase()
                      ? "text-primary font-bold bg-gray-50"
                      : "text-gray-600 hover:bg-gray-50"
                  }`}
                >
                  {tab}
                </button>
              )
            )}
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <div className="container mx-auto px-3 py-6">{children}</div>

      {/* Status Bar */}
      <div className="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200 px-3 py-2">
        <div className="container mx-auto flex items-center space-x-2">
          <span className="text-green-500 text-lg">●</span>
          <span className="text-sm font-medium text-green-500">
            qBittorrent Connected
          </span>
        </div>
      </div>

      {/* Click Outside Search Handler */}
      {isSearchExpanded && (
        <div
          className="fixed inset-0 bg-transparent"
          onClick={() => setIsSearchExpanded(false)}
        />
      )}
    </div>
  );
}
