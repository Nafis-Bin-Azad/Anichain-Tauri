"use client";

import { useState, useEffect } from "react";
import { invokeTauri } from "@/lib/tauri";
import { Loader2 } from "lucide-react";

interface QBitTorrentRule {
  name: string;
  pattern: string;
  save_path: string;
  enabled: boolean;
}

export default function Settings() {
  const [url, setUrl] = useState("http://localhost:8080");
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [rules, setRules] = useState<QBitTorrentRule[]>([]);

  useEffect(() => {
    loadRules();
  }, []);

  const loadRules = async () => {
    try {
      const rules = await invokeTauri<QBitTorrentRule[]>(
        "get_qbittorrent_rules"
      );
      setRules(rules);
    } catch (err) {
      console.error("Failed to load qBittorrent rules:", err);
    }
  };

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      await invokeTauri("initialize_qbittorrent", {
        url,
        username,
        password,
      });
      setSuccess("Successfully connected to qBittorrent!");
      loadRules();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Settings</h1>
        <p className="mt-1 text-sm text-gray-600">
          Configure your qBittorrent connection and RSS rules
        </p>
      </div>

      <div className="bg-white shadow sm:rounded-lg">
        <div className="px-4 py-5 sm:p-6">
          <h3 className="text-lg font-medium leading-6 text-gray-900">
            qBittorrent Connection
          </h3>
          <div className="mt-2 max-w-xl text-sm text-gray-500">
            <p>Enter your qBittorrent WebUI credentials</p>
          </div>
          <form onSubmit={handleConnect} className="mt-5 space-y-4">
            <div>
              <label
                htmlFor="url"
                className="block text-sm font-medium text-gray-700"
              >
                WebUI URL
              </label>
              <input
                type="url"
                name="url"
                id="url"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                placeholder="http://localhost:8080"
              />
            </div>
            <div>
              <label
                htmlFor="username"
                className="block text-sm font-medium text-gray-700"
              >
                Username
              </label>
              <input
                type="text"
                name="username"
                id="username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              />
            </div>
            <div>
              <label
                htmlFor="password"
                className="block text-sm font-medium text-gray-700"
              >
                Password
              </label>
              <input
                type="password"
                name="password"
                id="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              />
            </div>
            {error && (
              <div className="rounded-md bg-red-50 p-4">
                <div className="flex">
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-red-800">Error</h3>
                    <div className="mt-2 text-sm text-red-700">
                      <p>{error}</p>
                    </div>
                  </div>
                </div>
              </div>
            )}
            {success && (
              <div className="rounded-md bg-green-50 p-4">
                <div className="flex">
                  <div className="ml-3">
                    <p className="text-sm font-medium text-green-800">
                      {success}
                    </p>
                  </div>
                </div>
              </div>
            )}
            <button
              type="submit"
              disabled={loading}
              className={`inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 ${
                loading ? "opacity-75 cursor-not-allowed" : ""
              }`}
            >
              {loading ? (
                <Loader2 className="w-4 h-4 animate-spin mr-2" />
              ) : null}
              Connect
            </button>
          </form>
        </div>
      </div>

      {rules.length > 0 && (
        <div className="bg-white shadow sm:rounded-lg">
          <div className="px-4 py-5 sm:p-6">
            <h3 className="text-lg font-medium leading-6 text-gray-900">
              RSS Rules
            </h3>
            <div className="mt-4 space-y-4">
              {rules.map((rule) => (
                <div
                  key={rule.name}
                  className="border rounded-lg p-4 space-y-2"
                >
                  <h4 className="font-medium">{rule.name}</h4>
                  <p className="text-sm text-gray-600">
                    Pattern: {rule.pattern}
                  </p>
                  <p className="text-sm text-gray-600">
                    Save Path: {rule.save_path}
                  </p>
                  <div className="flex items-center space-x-2">
                    <div
                      className={`w-2 h-2 rounded-full ${
                        rule.enabled ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    <span className="text-sm text-gray-600">
                      {rule.enabled ? "Enabled" : "Disabled"}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
