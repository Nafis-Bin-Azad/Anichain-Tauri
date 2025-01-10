import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";

export interface AnimeEntry {
  title: string;
  link: string;
  date: string;
}

// Helper to check if running in Tauri context
export async function isTauriAvailable(): Promise<boolean> {
  console.log("🔍 Checking Tauri availability...");
  try {
    // Check if window exists
    if (typeof window === "undefined") {
      console.log("❌ Window object not available");
      return false;
    }

    console.log("✅ Window object exists, checking for Tauri...");

    // Check for various Tauri indicators
    const hasTauriGlobal = "window.__TAURI__" in window;
    const hasTauriIPC = "window.__TAURI_IPC__" in window;
    const hasTauriObject = Boolean((window as any).__TAURI__);

    console.log("Tauri detection results:", {
      hasTauriGlobal,
      hasTauriIPC,
      hasTauriObject,
    });

    if (hasTauriObject) {
      try {
        const version = await getVersion();
        console.log("✅ Tauri version detected:", version);
        return true;
      } catch (versionError) {
        console.error("❌ Failed to get Tauri version:", versionError);
      }
    }

    if (hasTauriIPC) {
      console.log("✅ Tauri IPC detected");
      return true;
    }

    console.log("❌ No Tauri context found");
    return false;
  } catch (error) {
    console.error("❌ Error checking Tauri availability:", error);
    return false;
  }
}

// Mock data for web environment
const mockData = {
  animeList: [
    { title: "Mock Anime 1", link: "#", date: "2024-01-09" },
    { title: "Mock Anime 2", link: "#", date: "2024-01-10" },
  ],
  trackedAnime: ["Mock Anime 1"],
  downloads: [
    {
      title: "Mock Anime 1",
      episode: "01",
      progress: 75,
      status: "downloading",
    },
  ],
  schedule: [
    {
      title: "Mock Anime 1",
      episode: "02",
      air_date: "Tomorrow",
    },
  ],
  settings: {
    rss_feed_url: "https://example.com/feed",
    download_path: "/downloads",
    qbittorrent_url: "http://localhost:8080",
    qbittorrent_username: "admin",
    qbittorrent_password: "",
  },
  animeDetails: {
    title: "Mock Anime",
    synopsis: "This is a mock anime for development.",
    image_url: "https://via.placeholder.com/300x450",
    episodes: 12,
    status: "Airing",
    score: 8.5,
    aired: "Winter 2024",
    genres: ["Action", "Adventure"],
  },
};

export async function invokeTauri<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  console.log(
    `🔄 Invoking Tauri command: ${command}`,
    args ? `with args: ${JSON.stringify(args)}` : "without args"
  );

  const isTauri = await isTauriAvailable();
  console.log(
    `📡 Tauri availability for ${command}:`,
    isTauri ? "✅ Available" : "❌ Not Available"
  );

  if (!isTauri) {
    console.warn(`⚠️ Using mock data for command: ${command}`);
    // Return mock data based on the command
    switch (command) {
      case "fetch_rss_feed":
        console.log("📦 Returning mock anime list");
        return mockData.animeList as T;
      case "get_tracked_anime":
        console.log("📦 Returning mock tracked anime");
        return mockData.trackedAnime as T;
      case "get_tracked_anime_details":
        console.log("📦 Returning mock tracked anime details");
        return mockData.animeList
          .filter((anime) => mockData.trackedAnime.includes(anime.title))
          .map((anime) => ({
            title: anime.title,
            episode: "01",
            image_path: null,
          })) as T;
      case "get_downloads":
        console.log("📦 Returning mock downloads");
        return mockData.downloads as T;
      case "get_schedule":
        console.log("📦 Returning mock schedule");
        return mockData.schedule as T;
      case "get_settings":
        console.log("📦 Returning mock settings");
        return mockData.settings as T;
      case "fetch_anime_details":
        console.log("📦 Returning mock anime details");
        return mockData.animeDetails as T;
      case "track_anime":
      case "untrack_anime":
      case "save_settings":
      case "cache_anime_image":
      case "remove_download":
        console.log("📦 Returning undefined for action command");
        return undefined as T;
      default:
        console.warn(`❌ No mock data available for command: ${command}`);
        return undefined as T;
    }
  }

  try {
    console.log(`🚀 Executing Tauri command: ${command}`, args);
    const result = await invoke(command, args);
    console.log(`✅ Tauri command successful: ${command}`, result);
    return result as T;
  } catch (error) {
    console.error(`❌ Error executing Tauri command ${command}:`, error);
    throw error;
  }
}

export async function fetchAnimeData() {
  console.log("📥 Starting anime data fetch process...");
  const isTauri = await isTauriAvailable();
  console.log(
    "🔍 Tauri context check result:",
    isTauri ? "✅ Available" : "❌ Not Available"
  );

  if (!isTauri) {
    console.warn("⚠️ Using mock data for anime fetch");
    const mockResult = {
      animeList: mockData.animeList,
      trackedAnime: mockData.trackedAnime,
    };
    console.log("📦 Returning mock data:", mockResult);
    return mockResult;
  }

  try {
    console.log("🔄 Fetching real anime data from Tauri backend...");
    const [feed, tracked] = await Promise.all([
      invokeTauri<AnimeEntry[]>("fetch_rss_feed"),
      invokeTauri<string[]>("get_tracked_anime"),
    ]);
    const result = {
      animeList: feed,
      trackedAnime: tracked,
    };
    console.log("✅ Real anime data fetch complete:", result);
    return result;
  } catch (error) {
    console.error("❌ Failed to fetch real anime data:", error);
    throw error;
  }
}
