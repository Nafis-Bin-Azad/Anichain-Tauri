use std::path::Path;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::fs;
use tracing::{debug, error, info, warn};
use reqwest;
use urlencoding;

lazy_static! {
    // Updated regex patterns based on HAMA's conventions
    static ref EPISODE_PATTERNS: Vec<(&'static str, Regex)> = vec![
        // AniDB style: [Group] Show Title - 01 (1080p) [hash].mkv
        ("AniDB", Regex::new(
            r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*[-\[]|\s+(?:EP|Episode|第)\s*)?(?P<episode>\d{1,3})(?:v\d)?(?:\s*(?:\[(?:[^\]]*)\]|\([^\)]*\)))*"
        ).unwrap()),
        
        // TVDB style: Show.Title.S01E01.Episode.Title.mkv
        ("TVDB", Regex::new(
            r"(?ix)(?P<title>.+?)(?:\.|\s+)?(?:S(?P<season>\d{1,2})?(?:E|x)(?P<episode>\d{1,2}))"
        ).unwrap()),
        
        // Absolute style: Show Title - 01.mkv
        ("Absolute", Regex::new(
            r"(?ix)(?P<title>[^-]+?)\s*[-\s.]+\s*(?P<episode>\d{1,3})(?:v\d)?"
        ).unwrap()),
        
        // Special episodes
        ("Special", Regex::new(
            r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*[-\[]|\s+)?(?P<special>S(?:pecial)?|OVA|OP|ED|NCOP|NCED|Preview|Movie)(?:\s*[-\[]|\s+)?(?P<episode>\d{1,3})?"
        ).unwrap()),
        
        // Movie pattern
        ("Movie", Regex::new(
            r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*(?:Movie|劇場版|完全版))?(?:\s*[-\[]|\s+)?(?:\s*\((?P<year>\d{4})\))?"
        ).unwrap()),
    ];

    // Folder patterns
    static ref FOLDER_PATTERNS: Vec<(&'static str, Regex)> = vec![
        // AniDB style: [Group] Show Title (Year)
        ("AniDB", Regex::new(
            r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*\((?P<year>\d{4})\))?(?:\s*\[(?P<source>BD|DVD|WEB)\])?"
        ).unwrap()),
        
        // Season style: Show Title Season 01
        ("Season", Regex::new(
            r"(?ix)(?P<title>.+?)(?:\s+(?:Season|S)\s*(?P<season>\d{1,2}))"
        ).unwrap()),
        
        // Year style: Show Title (2024)
        ("Year", Regex::new(
            r"(?ix)(?P<title>.+?)\s*\((?P<year>\d{4})\)"
        ).unwrap()),
        
        // Metadata ID style: Show Title [anidb-12345]
        ("MetadataID", Regex::new(
            r"(?ix)(?P<title>.+?)\s*\[(?:anidb|tvdb)-(?P<id>\d+)\]"
        ).unwrap()),
    ];

    // Quality pattern
    static ref QUALITY_PATTERN: Regex = Regex::new(
        r"(?i)(?:\[|\()(?P<quality>(?:\d{3,4}[pi]|SD|HD|FHD|UHD|4K|8K|BD|DVD|WEB)(?:-(?:Hi10P|10bit|HEVC|H\.?265|x265|AVC|H\.?264|x264))?)(?:\]|\))"
    ).unwrap();
}

#[derive(Debug, Serialize, Clone)]
pub struct AnimeEpisode {
    pub filename: String,
    pub title: String,
    pub episode_number: Option<String>,
    pub is_special: bool,
    pub quality: Option<String>,
    pub size: String,
    pub path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnimeFolder {
    pub folder_name: String,
    pub title: String,
    pub season: Option<String>,
    pub episodes: Vec<AnimeEpisode>,
    pub specials: Vec<AnimeEpisode>,
    pub path: String,
    pub image_url: Option<String>,
}

/// Scans the given root folder recursively and returns a list of AnimeFolder objects.
#[allow(dead_code)]
pub async fn scan_anime_folder(root_path: &str) -> Result<Vec<AnimeFolder>> {
    let mut anime_folders = Vec::new();
    let root = Path::new(root_path);

    if !root.exists() {
        return Ok(anime_folders);
    }

    // Create a reqwest client for fetching images.
    let client = reqwest::Client::new();

    // Recursively scan the root directory.
    scan_directory(root, &mut anime_folders)?;

    // Sort folders by title.
    anime_folders.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    // Fetch images for each folder.
    for folder in &mut anime_folders {
        if let Some(image_url) = fetch_anime_image(&client, &folder.title).await {
            folder.image_url = Some(image_url);
        }
    }

    // Sort episodes within each folder.
    for folder in &mut anime_folders {
        // Sort regular episodes by episode number.
        folder.episodes.sort_by(|a, b| {
            let a_num = a.episode_number.as_ref().and_then(|n| n.parse::<i32>().ok()).unwrap_or(0);
            let b_num = b.episode_number.as_ref().and_then(|n| n.parse::<i32>().ok()).unwrap_or(0);
            a_num.cmp(&b_num)
        });

        // Sort specials by filename.
        folder.specials.sort_by(|a, b| a.filename.cmp(&b.filename));
    }

    Ok(anime_folders)
}

/// Fetches an anime image URL using the Jikan API given an anime title.
#[allow(dead_code)]
async fn fetch_anime_image(client: &reqwest::Client, title: &str) -> Option<String> {
    // Clean up the title.
    let clean_title = title.replace("[SubsPlease]", "")
                           .split(" - ").next()?
                           .split("[").next()?
                           .trim()
                           .to_string();

    info!("Fetching image for anime: {}", clean_title);

    // Search for the anime using the Jikan API.
    let query_url = format!(
        "https://api.jikan.moe/v4/anime?q={}&limit=1",
        urlencoding::encode(&clean_title)
    );

    debug!("Making request to Jikan API: {}", query_url);

    match client.get(&query_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                error!("Failed to fetch anime image. Status: {}", response.status());
                return None;
            }
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    if let Some(results) = data.get("data") {
                        if let Some(results_array) = results.as_array() {
                            if let Some(first_result) = results_array.first() {
                                if let Some(images) = first_result.get("images") {
                                    if let Some(jpg) = images.get("jpg") {
                                        if let Some(url) = jpg.get("large_image_url") {
                                            if let Some(url_str) = url.as_str() {
                                                info!("Successfully found image for '{}': {}", clean_title, url_str);
                                                return Some(url_str.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    warn!("No image found in API response for: {}", clean_title);
                }
                Err(e) => {
                    error!("Failed to parse API response: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch anime image: {}", e);
        }
    }
    None
}

/// Recursively scans a directory for video files and fills the provided folders vector.
#[allow(dead_code)]
fn scan_directory(dir: &Path, folders: &mut Vec<AnimeFolder>) -> Result<()> {
    debug!("Scanning directory: {}", dir.display());

    let mut current_folder = AnimeFolder {
        folder_name: dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        title: String::new(),
        season: None,
        episodes: Vec::new(),
        specials: Vec::new(),
        path: dir.to_string_lossy().to_string(),
        image_url: None,
    };

    // Try to parse the folder name using different patterns.
    if let Some(folder_name) = dir.file_name().map(|n| n.to_string_lossy()) {
        debug!("Parsing folder name: '{}'", folder_name);
        
        // Special handling for numeric-only folder names.
        if folder_name.trim().chars().all(|c| c.is_numeric()) {
            let series_info_path = dir.join("series.json");
            let info_path = dir.join("info.json");
            
            if series_info_path.exists() {
                if let Ok(contents) = fs::read_to_string(&series_info_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                        if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
                            current_folder.title = title.to_string();
                        }
                    }
                }
            } else if info_path.exists() {
                if let Ok(contents) = fs::read_to_string(&info_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                        if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
                            current_folder.title = title.to_string();
                        }
                    }
                }
            } else {
                // If no info file exists, try to infer from video file names.
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if file_name.ends_with(".mkv") || file_name.ends_with(".mp4") {
                                if let Some(series_name) = extract_series_name_from_filename(file_name) {
                                    current_folder.title = series_name;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let mut found_match = false;
        if current_folder.title.is_empty() {
            for (pattern_name, pattern) in FOLDER_PATTERNS.iter() {
                if let Some(caps) = pattern.captures(&folder_name) {
                    debug!("Matched {} pattern", pattern_name);
                    
                    // Extract title and clean it.
                    let mut title = caps.name("title").map_or("", |m| m.as_str()).trim().to_string();
                    title = title.replace(".", " ").replace("_", " ");
                    if !title.is_empty() {
                        current_folder.title = title;
                    }

                    // Handle season information.
                    if let Some(season) = caps.name("season") {
                        current_folder.season = Some(season.as_str().to_string());
                    } else if let Some(year) = caps.name("year") {
                        current_folder.season = Some(format!("({})", year.as_str()));
                    }

                    found_match = true;
                    break;
                }
            }
        }

        if !found_match && current_folder.title.is_empty() {
            debug!("No folder pattern matched, using raw folder name");
            current_folder.title = folder_name.to_string();
        }
    }

    let mut has_content = false;

    // Process all files in the directory.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry_result in entries.flatten() {
            let path = entry_result.path();
            if let Ok(metadata) = entry_result.metadata() {
                if metadata.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ["mp4", "mkv", "avi", "m4v"].contains(&ext.as_str()) {
                            if let Some(filename) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
                                debug!("Processing video file: '{}'", filename);
                                
                                let size = metadata.len();
                                let size_str = if size < 1024 {
                                    format!("{}B", size)
                                } else if size < 1024 * 1024 {
                                    format!("{:.1}KB", size as f64 / 1024.0)
                                } else if size < 1024 * 1024 * 1024 {
                                    format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
                                } else {
                                    format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                                };

                                let mut episode_found = false;
                                // Try each pattern until a match is found.
                                for (pattern_name, pattern) in EPISODE_PATTERNS.iter() {
                                    if let Some(caps) = pattern.captures(&filename) {
                                        debug!("Matched {} pattern for file", pattern_name);
                                        
                                        // Extract quality using the separate quality pattern.
                                        let quality = QUALITY_PATTERN.captures(&filename)
                                            .and_then(|q| q.name("quality"))
                                            .map(|m| m.as_str().to_string());

                                        let mut episode = AnimeEpisode {
                                            filename: filename.clone(),
                                            title: caps.name("title")
                                                .map_or(filename.as_str(), |m| m.as_str())
                                                .trim()
                                                .replace(".", " ")
                                                .replace("_", " "),
                                            episode_number: caps.name("episode").map(|m| m.as_str().to_string()),
                                            is_special: caps.name("special").is_some(),
                                            quality,
                                            size: size_str.clone(),
                                            path: path.to_string_lossy().to_string(),
                                        };

                                        // If no title was found in the episode, use the folder title.
                                        if episode.title.is_empty() && !current_folder.title.is_empty() {
                                            episode.title = current_folder.title.clone();
                                        }

                                        // Update folder title if empty.
                                        if current_folder.title.is_empty() && !episode.title.is_empty() {
                                            current_folder.title = episode.title.clone();
                                        }

                                        if episode.is_special {
                                            current_folder.specials.push(episode);
                                        } else {
                                            current_folder.episodes.push(episode);
                                        }
                                        has_content = true;
                                        episode_found = true;
                                        break;
                                    }
                                }

                                if !episode_found {
                                    debug!("No pattern matched for file: '{}'", filename);
                                }
                            }
                        }
                    }
                } else if metadata.is_dir() {
                    scan_directory(&path, folders)?;
                }
            }
        }
    }

    if has_content {
        debug!(
            "Adding folder '{}' with {} episodes and {} specials",
            current_folder.title,
            current_folder.episodes.len(),
            current_folder.specials.len()
        );
        folders.push(current_folder);
    }

    Ok(())
}

/// Attempts to extract a series name from a filename using common patterns.
#[allow(dead_code)]
fn extract_series_name_from_filename(filename: &str) -> Option<String> {
    lazy_static! {
        static ref PATTERNS: Vec<Regex> = vec![
            Regex::new(r"(?i)\[(?:[^\]]+)\](?:\s*)(.*?)(?:\s*-\s*\d+)").unwrap(),  // [Group] Series Name - 01
            Regex::new(r"(?i)(.*?)\s*-\s*(?:Episode\s*)?\d+").unwrap(),             // Series Name - 01 or Series Name - Episode 01
            Regex::new(r"(?i)(.*?)\s*(?:E|Ep|Episode)\s*\d+").unwrap(),              // Series Name E01 or Series Name Episode 01
        ];
    }

    for pattern in PATTERNS.iter() {
        if let Some(caps) = pattern.captures(filename) {
            if let Some(series_name) = caps.get(1) {
                let name = series_name.as_str().trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }

    None
}
