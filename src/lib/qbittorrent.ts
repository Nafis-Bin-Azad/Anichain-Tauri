import { invoke } from "@tauri-apps/api/core";

export interface TorrentInfo {
  name: string;
  size: number;
  progress: number;
  download_speed: number;
  state: string;
}

export interface QBittorrentConfig {
  url: string;
  username: string;
  password: string;
}

class QBittorrentClient {
  private static instance: QBittorrentClient;
  private isConnected: boolean = false;

  private constructor() {}

  public static getInstance(): QBittorrentClient {
    if (!QBittorrentClient.instance) {
      QBittorrentClient.instance = new QBittorrentClient();
    }
    return QBittorrentClient.instance;
  }

  async connect(config: QBittorrentConfig): Promise<void> {
    try {
      await invoke("connect_qbittorrent", {
        url: config.url,
        username: config.username,
        password: config.password,
      });
      this.isConnected = true;
    } catch (error) {
      console.error("Failed to connect to qBittorrent:", error);
      this.isConnected = false;
      throw error;
    }
  }

  async checkConnection(): Promise<boolean> {
    try {
      this.isConnected = await invoke("check_qbittorrent_connection");
      return this.isConnected;
    } catch (error) {
      console.error("Failed to check qBittorrent connection:", error);
      this.isConnected = false;
      return false;
    }
  }

  async getTorrents(): Promise<TorrentInfo[]> {
    if (!this.isConnected) {
      throw new Error("Not connected to qBittorrent");
    }

    try {
      return await invoke<TorrentInfo[]>("get_torrents");
    } catch (error) {
      console.error("Failed to get torrents:", error);
      throw error;
    }
  }

  async addTorrent(magnetUrl: string): Promise<void> {
    if (!this.isConnected) {
      throw new Error("Not connected to qBittorrent");
    }

    try {
      await invoke("add_torrent", { magnetUrl });
    } catch (error) {
      console.error("Failed to add torrent:", error);
      throw error;
    }
  }

  async removeTorrent(
    hash: string,
    deleteFiles: boolean = false
  ): Promise<void> {
    if (!this.isConnected) {
      throw new Error("Not connected to qBittorrent");
    }

    try {
      await invoke("remove_torrent", { hash, deleteFiles });
    } catch (error) {
      console.error("Failed to remove torrent:", error);
      throw error;
    }
  }

  async pauseTorrent(hash: string): Promise<void> {
    if (!this.isConnected) {
      throw new Error("Not connected to qBittorrent");
    }

    try {
      await invoke("pause_torrent", { hash });
    } catch (error) {
      console.error("Failed to pause torrent:", error);
      throw error;
    }
  }

  async resumeTorrent(hash: string): Promise<void> {
    if (!this.isConnected) {
      throw new Error("Not connected to qBittorrent");
    }

    try {
      await invoke("resume_torrent", { hash });
    } catch (error) {
      console.error("Failed to resume torrent:", error);
      throw error;
    }
  }

  getConnectionStatus(): boolean {
    return this.isConnected;
  }
}

export const qbittorrent = QBittorrentClient.getInstance();
