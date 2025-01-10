import { invoke as mockInvoke } from "./mockTauri";

let invoke = mockInvoke;

// Try to initialize Tauri in runtime
if (typeof window !== "undefined") {
  import("@tauri-apps/api/tauri")
    .then((tauri) => {
      invoke = tauri.invoke;
    })
    .catch((error) => {
      console.warn("Failed to initialize Tauri:", error);
    });
}

export interface AnimeEntry {
  title: string;
  link: string;
  date: string;
  image_url?: string;
  summary?: string;
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

export async function isTauriAvailable(): Promise<boolean> {
  try {
    await invoke("get_tracked_anime");
    return true;
  } catch (error) {
    return false;
  }
}

export async function invokeTauri<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    console.log(
      `🔄 Invoking Tauri command: ${command}`,
      args ? `with args: ${JSON.stringify(args)}` : "without args"
    );

    // Check if we're in a Tauri environment
    const isTauri = await isTauriAvailable();
    console.log(
      `📡 Tauri availability for ${command}:`,
      isTauri ? "✅ Available" : "❌ Not Available"
    );

    if (!isTauri) {
      console.warn(`⚠️ Using mock data for command: ${command}`);
      switch (command) {
        case "fetch_rss_feed":
          console.log("📦 Returning mock anime list");
          return mockData.animeList as T;
        case "get_tracked_anime":
          console.log("📦 Returning mock tracked anime");
          return mockData.trackedAnime as T;
        case "get_schedule":
          return mockData.schedule as T;
        default:
          return {} as T;
      }
    }

    // If we're in Tauri, make the actual call
    const result = await invoke(command, args);
    return result as T;
  } catch (error) {
    console.error(`❌ Error invoking ${command}:`, error);
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
