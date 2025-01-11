export interface Anime {
  title: string;
  episodeInfo: string;
  imageUrl: string;
  description: string;
  status: "Ongoing" | "Ended";
  lastEpisode: string;
  nextEpisodeTime?: Date;
}

export interface Download {
  filename: string;
  imageUrl: string;
  progress: number;
  isDownloading: boolean;
  fileSize: string;
}

export const mockAnimeList: Anime[] = [
  {
    title: "Jujutsu Kaisen Season 2",
    episodeInfo: "Episode 23",
    imageUrl: "https://cdn.myanimelist.net/images/anime/1600/134703.jpg",
    description:
      "The hidden world of Jujutsu Sorcery continues as Yuji Itadori and his friends face new challenges and powerful curses.",
    status: "Ongoing",
    lastEpisode: "Episode 23",
    nextEpisodeTime: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000), // 7 days from now
  },
  {
    title: "Solo Leveling",
    episodeInfo: "Episode 1",
    imageUrl: "https://cdn.myanimelist.net/images/anime/1926/135795.jpg",
    description:
      "In a world where hunters must battle deadly monsters to protect humanity, Sung Jinwoo is known as the weakest of all hunters.",
    status: "Ongoing",
    lastEpisode: "Episode 1",
    nextEpisodeTime: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000), // 3 days from now
  },
  {
    title: "Demon Slayer",
    episodeInfo: "Season 3 Complete",
    imageUrl: "https://cdn.myanimelist.net/images/anime/1908/135431.jpg",
    description:
      "Tanjiro's journey continues as he faces powerful demons while trying to turn his sister back into a human.",
    status: "Ended",
    lastEpisode: "Episode 11",
  },
];

export const mockDownloads: Download[] = [
  {
    filename: "Jujutsu Kaisen S2E23.mkv",
    imageUrl: "https://cdn.myanimelist.net/images/anime/1600/134703.jpg",
    progress: 75,
    isDownloading: true,
    fileSize: "1.2 GB",
  },
  {
    filename: "Solo Leveling E01.mkv",
    imageUrl: "https://cdn.myanimelist.net/images/anime/1926/135795.jpg",
    progress: 100,
    isDownloading: false,
    fileSize: "850 MB",
  },
];
