"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Home, Calendar, BookMarked, Download, Settings } from "lucide-react";

export default function Navbar() {
  const pathname = usePathname();

  const isActive = (path: string) => pathname === path;

  const navItems = [
    { href: "/", icon: Home, label: "Home" },
    { href: "/schedule", icon: Calendar, label: "Schedule" },
    { href: "/tracked", icon: BookMarked, label: "Tracked" },
    { href: "/downloads", icon: Download, label: "Downloads" },
    { href: "/settings", icon: Settings, label: "Settings" },
  ];

  return (
    <nav className="fixed top-0 left-0 right-0 h-16 bg-white border-b border-gray-200 z-50">
      <div className="h-full max-w-7xl mx-auto px-4 flex items-center justify-between">
        <div className="flex items-center space-x-1">
          <span className="text-xl font-bold text-gray-900">ANICHAIN</span>
        </div>
        <div className="flex items-center space-x-4">
          {navItems.map(({ href, icon: Icon, label }) => (
            <Link
              key={href}
              href={href}
              className={`flex items-center space-x-1 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                isActive(href)
                  ? "bg-gray-100 text-gray-900"
                  : "text-gray-600 hover:bg-gray-50 hover:text-gray-900"
              }`}
            >
              <Icon className="w-4 h-4" />
              <span>{label}</span>
            </Link>
          ))}
        </div>
      </div>
    </nav>
  );
}
