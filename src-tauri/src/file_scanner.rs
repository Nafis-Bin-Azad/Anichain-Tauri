// src/file_scanner.rs

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use log::{debug, error, info, warn};
use reqwest;
use urlencoding::encode;

/// Returns a slice of supported video file extensions.
pub fn video_exts() -> &'static [&'static str] {
    &[
        "3g2", "3gp", "asf", "asx", "avc", "avi", "avs", "bin", "bivx", "divx", "dv", "dvr-ms", "evo",
        "fli", "flv", "img", "iso", "m2t", "m2ts", "m2v", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg",
        "mts", "nrg", "nsv", "nuv", "ogm", "ogv", "tp", "pva", "qt", "rm", "rmvb", "sdp", "swf",
        "svq3", "strm", "ts", "ty", "vdr", "viv", "vp3", "wmv", "wpl", "wtv", "xsp", "xvid", "webm",
        "ifo", "disc",
    ]
}

lazy_static! {
    // These regexes try to capture episode information from filenames.
    static ref EPISODE_PATTERNS: Vec<(&'static str, Regex)> = vec![
        // AniDB style: [Group] Show Title - 01 (1080p) [hash].mkv
        (
            "AniDB",
            Regex::new(
                r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*[-\[]|\s+(?:EP|Episode|第)\s*)?(?P<episode>\d{1,3})(?:v\d)?(?:\s*(?:\[[^\]]+\]|\([^\)]+\)))*"
            ).unwrap()
        ),
        // TVDB style: Show.Title.S01E01.Episode.Title.mkv
        (
            "TVDB",
            Regex::new(
                r"(?ix)(?P<title>.+?)(?:\.|\s+)?(?:S(?P<season>\d{1,2})?(?:E|x)(?P<episode>\d{1,2}))"
            ).unwrap()
        ),
        // Absolute style: Show Title - 01.mkv
        (
            "Absolute",
            Regex::new(
                r"(?ix)(?P<title>[^-]+?)\s*[-\s.]+\s*(?P<episode>\d{1,3})(?:v\d)?"
            ).unwrap()
        ),
        // Special episodes (e.g., OP, ED, OVA)
        (
            "Special",
            Regex::new(
                r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*[-\[]|\s+)?(?P<special>S(?:pecial)?|OVA|OP|ED|NCOP|NCED|Preview|Movie)(?:\s*[-\[]|\s+)?(?P<episode>\d{1,3})?"
            ).unwrap()
        ),
        // Movie pattern (for standalone movies)
        (
            "Movie",
            Regex::new(
                r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*(?:Movie|劇場版|完全版))?(?:\s*[-\[]|\s+)?(?:\s*\((?P<year>\d{4})\))?"
            ).unwrap()
        ),
    ];

    // These regexes try to extract folder metadata.
    static ref FOLDER_PATTERNS: Vec<(&'static str, Regex)> = vec![
        (
            "AniDB",
            Regex::new(
                r"(?ix)(?:\[(?P<group>[^\]]+)\])?[^\[]*?(?P<title>[^-\[]+?)(?:\s*\((?P<year>\d{4})\))?(?:\s*\[(?P<source>BD|DVD|WEB)\])?"
            ).unwrap()
        ),
        (
            "Season",
            Regex::new(
                r"(?ix)(?P<title>.+?)(?:\s+(?:Season|S)\s*(?P<season>\d{1,2}))"
            ).unwrap()
        ),
        (
            "Year",
            Regex::new(
                r"(?ix)(?P<title>.+?)\s*\((?P<year>\d{4})\)"
            ).unwrap()
        ),
        (
            "MetadataID",
            Regex::new(
                r"(?ix)(?P<title>.+?)\s*\[(?:anidb|tvdb)-(?P<id>\d+)\]"
            ).unwrap()
        ),
    ];

    // Quality pattern: extract quality strings (e.g., 1080p, 720p)
    static ref QUALITY_PATTERN: Regex = Regex::new(
        r"(?i)(?:\[|\()(?P<quality>(?:\d{3,4}[pi]|SD|HD|FHD|UHD|4K|8K|BD|DVD|WEB)(?:-(?:Hi10P|10bit|HEVC|H\.?265|x265|AVC|H\.?264|x264))?)(?:\]|\))"
    ).unwrap();
}

/// Cleans a string by removing parenthesized and bracketed text and extra whitespace.
/// (A simplified version of Python’s clean_string.)
pub fn clean_string(s: &str) -> String {
    // Remove text inside parentheses and square brackets.
    let re_paren = Regex::new(r"\([^\(\)]*\)").unwrap();
    let re_bracket = Regex::new(r"\[[^\[\]]*\]").unwrap();
    let mut cleaned = re_paren.replace_all(s, " ").to_string();
    cleaned = re_bracket.replace_all(&cleaned, " ").to_string();
    // Replace multiple spaces with a single space.
    let re_space = Regex::new(r"\s+").unwrap();
    cleaned = re_space.replace_all(&cleaned, " ").to_string();
    cleaned.trim().to_string()
}

/// Splits a string into a vector of “chunks” for natural sorting.
/// (This is a simple implementation that splits on digits.)
pub fn natural_sort_key(s: &str) -> Vec<String> {
    let re = Regex::new(r"(\d+)").unwrap();
    re.split(s).map(|x| x.to_lowercase()).collect()
}

/// Converts a Roman numeral string to an integer.
pub fn roman_to_int(s: &str) -> Option<u32> {
    let roman_map = [
        ("CM", 900), ("M", 1000), ("CD", 400), ("D", 500),
        ("XC", 90), ("C", 100), ("XL", 40), ("L", 50),
        ("IX", 9), ("X", 10), ("IV", 4), ("V", 5), ("I", 1),
    ];
    let mut i = 0;
    let s = s.to_uppercase();
    let mut num = 0;
    while i < s.len() {
        let mut matched = false;
        for &(roman, value) in &roman_map {
            if s[i..].starts_with(roman) {
                num += value;
                i += roman.len();
                matched = true;
                break;
            }
        }
        if !matched {
            return None;
        }
    }
    Some(num)
}

/// Attempts to extract a series name from a filename.
pub fn extract_series_name_from_filename(filename: &str) -> Option<String> {
    lazy_static! {
        static ref PATTERNS: Vec<Regex> = vec![
            Regex::new(r"(?i)\[(?:[^\]]+)\]\s*(.*?)(?:\s*-\s*\d+)").unwrap(),
            Regex::new(r"(?i)(.*?)\s*-\s*(?:Episode\s*)?\d+").unwrap(),
            Regex::new(r"(?i)(.*?)\s*(?:E|Ep|Episode)\s*\d+").unwrap(),
        ];
    }
    for pat in PATTERNS.iter() {
        if let Some(caps) = pat.captures(filename) {
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

/// Represents a scanned anime episode.
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

/// Represents a folder containing anime episodes.
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

/// Recursively scans the given root folder and returns a vector of AnimeFolder objects.
pub async fn scan_anime_folder(root_path: &str) -> Result<Vec<AnimeFolder>> {
    let mut folders = Vec::new();
    let root = Path::new(root_path);
    if !root.exists() {
        return Ok(folders);
    }
    // Recursively scan the directory.
    scan_directory(root, &mut folders)?;
    // Sort folders by title.
    folders.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    // Create a reqwest client for fetching images.
    let client = reqwest::Client::new();
    for folder in &mut folders {
        if let Some(url) = fetch_anime_image(&client, &folder.title).await {
            folder.image_url = Some(url);
        }
        // Sort episodes (if episode_number parses as an integer).
        folder.episodes.sort_by(|a, b| {
            let a_num = a.episode_number.as_ref().and_then(|n| n.parse::<i32>().ok()).unwrap_or(0);
            let b_num = b.episode_number.as_ref().and_then(|n| n.parse::<i32>().ok()).unwrap_or(0);
            a_num.cmp(&b_num)
        });
        folder.specials.sort_by(|a, b| a.filename.cmp(&b.filename));
    }
    Ok(folders)
}

/// Fetches an anime image URL using the Jikan API (v4) given a title.
async fn fetch_anime_image(client: &reqwest::Client, title: &str) -> Option<String> {
    // Clean the title: remove known substrings and take the first part.
    let clean_title = title
        .replace("[SubsPlease]", "")
        .split(" - ")
        .next()?
        .split("[")
        .next()?
        .trim()
        .to_string();
    info!("Fetching image for anime: {}", clean_title);
    let query_url = format!(
        "https://api.jikan.moe/v4/anime?q={}&limit=1",
        encode(&clean_title)
    );
    debug!("Requesting Jikan API: {}", query_url);
    match client.get(&query_url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                error!("Jikan API request failed with status: {}", resp.status());
                return None;
            }
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(results) = data.get("data") {
                    if let Some(arr) = results.as_array() {
                        if let Some(first) = arr.first() {
                            if let Some(images) = first.get("images") {
                                if let Some(jpg) = images.get("jpg") {
                                    if let Some(url) = jpg.get("large_image_url") {
                                        if let Some(url_str) = url.as_str() {
                                            info!("Found image for '{}': {}", clean_title, url_str);
                                            return Some(url_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                warn!("No image found in Jikan response for: {}", clean_title);
            }
            None
        },
        Err(e) => {
            error!("Failed to fetch image from Jikan: {}", e);
            None
        }
    }
}

/// Recursively scans a directory and populates the provided folders vector with AnimeFolder objects.
fn scan_directory(dir: &Path, folders: &mut Vec<AnimeFolder>) -> Result<()> {
    debug!("Scanning directory: {}", dir.display());

    // Initialize a new folder for the current directory.
    let mut current_folder = AnimeFolder {
        folder_name: dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        title: String::new(),
        season: None,
        episodes: Vec::new(),
        specials: Vec::new(),
        path: dir.to_string_lossy().to_string(),
        image_url: None,
    };

    // Attempt to parse the folder name using the folder patterns.
    if let Some(folder_name) = dir.file_name().map(|n| n.to_string_lossy()) {
        debug!("Parsing folder name: '{}'", folder_name);
        let mut found_match = false;
        for (pat_name, pat) in FOLDER_PATTERNS.iter() {
            if let Some(caps) = pat.captures(&folder_name) {
                debug!("Folder pattern '{}' matched", pat_name);
                let title = caps
                    .name("title")
                    .map_or("", |m| m.as_str())
                    .trim()
                    .replace(".", " ")
                    .replace("_", " ");
                if !title.is_empty() {
                    current_folder.title = title;
                }
                if let Some(season) = caps.name("season") {
                    current_folder.season = Some(season.as_str().to_string());
                } else if let Some(year) = caps.name("year") {
                    current_folder.season = Some(format!("({})", year.as_str()));
                }
                found_match = true;
                break;
            }
        }
        if !found_match && current_folder.title.is_empty() {
            debug!("No folder pattern matched; using raw folder name");
            current_folder.title = folder_name.to_string();
        }
    }

    let mut has_content = false;
    // Process each entry in the current directory.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(ext_osstr) = path.extension() {
                        let ext = ext_osstr.to_string_lossy().to_lowercase();
                        if video_exts().contains(&ext.as_str()) {
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
                                // Try matching each episode pattern.
                                for (pat_name, pat) in EPISODE_PATTERNS.iter() {
                                    if let Some(caps) = pat.captures(&filename) {
                                        debug!("File '{}' matched pattern '{}'", filename, pat_name);
                                        let quality = QUALITY_PATTERN.captures(&filename)
                                            .and_then(|q| q.name("quality"))
                                            .map(|m| m.as_str().to_string());
                                        let episode = AnimeEpisode {
                                            filename: filename.clone(),
                                            title: caps
                                                .name("title")
                                                .map_or(&filename, |m| m.as_str())
                                                .trim()
                                                .replace(".", " ")
                                                .replace("_", " "),
                                            episode_number: caps.name("episode").map(|m| m.as_str().to_string()),
                                            is_special: caps.name("special").is_some(),
                                            quality,
                                            size: size_str.clone(),
                                            path: path.to_string_lossy().to_string(),
                                        };

                                        if episode.title.is_empty() && !current_folder.title.is_empty() {
                                            // Fallback: use the folder title if episode title is empty.
                                            let mut ep = episode.clone();
                                            ep.title = current_folder.title.clone();
                                            if ep.is_special {
                                                current_folder.specials.push(ep);
                                            } else {
                                                current_folder.episodes.push(ep);
                                            }
                                        } else {
                                            if episode.is_special {
                                                current_folder.specials.push(episode);
                                            } else {
                                                current_folder.episodes.push(episode);
                                            }
                                        }
                                        has_content = true;
                                        episode_found = true;
                                        break;
                                    }
                                }
                                if !episode_found {
                                    debug!("No episode pattern matched for file: '{}'", filename);
                                }
                            }
                        }
                    }
                } else if metadata.is_dir() {
                    // Recurse into subdirectories.
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
