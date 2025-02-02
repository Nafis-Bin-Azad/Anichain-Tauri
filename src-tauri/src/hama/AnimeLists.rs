use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use xmltree::Element;
use log::{info, debug, error};

//
// Constants and URLs
//
lazy_static! {
    static ref SCUDLEE_MASTER: String = "https://raw.githubusercontent.com/Anime-Lists/anime-lists/master/anime-list-master.xml".to_string();
    static ref SCUDLEE_MOVIESET: String = "https://raw.githubusercontent.com/Anime-Lists/anime-lists/master/anime-movieset-list.xml".to_string();
    static ref SCUDLEE_CUSTOM: String = "anime-list-custom.xml".to_string();
    static ref SCUDLEE_FEEDBACK: String = "https://github.com/Anime-Lists/anime-lists/issues/new?template=new_mapping.yml&title={title}&anidb_id={anidb_id}&anime_title={anidb_title}".to_string();
}

/// Helper function to get a nested value from a JSON object by a list of keys.
fn get_nested<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current)
}

/// Helper function to set a nested value in a JSON object. Intermediate objects are created as needed.
fn set_nested(value: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &keys[..keys.len()-1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len()-1]] = new_value;
}

/// MergeMaps: Deep-copy the master mapping and then replace any anime node whose “anidbid” is present in the fix mapping.
pub fn merge_maps(master: &[Element], fix: Option<&Element>) -> Vec<Element> {
    // Deep copy master
    let mut new_map = master.to_vec();
    let mut fix_nodes: HashMap<String, Element> = HashMap::new();
    if let Some(fix_elem) = fix {
        for node in &fix_elem.children {
            if let Some(id) = node.attributes.get("anidbid") {
                fix_nodes.insert(id.clone(), node.clone());
            }
        }
        info!("MergeMaps() - AniDBids concerned: {:?}", fix_nodes.keys());
    }
    // Remove nodes from new_map that appear in fix_nodes.
    new_map.retain(|node| {
        if let Some(id) = node.attributes.get("anidbid") {
            !fix_nodes.contains_key(id)
        } else {
            true
        }
    });
    // Append the fix nodes.
    for node in fix_nodes.values() {
        new_map.push(node.clone());
    }
    new_map
}

/// Load the AniDBTVDBMap from the SCUDLEE_MASTER URL (or local file).
pub fn get_anidb_tvdb_map() -> Result<Vec<Element>> {
    let filename = Path::new(&*SCUDLEE_MASTER)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid SCUDLEE_MASTER URL"))?;
    let local_path = Path::new("AnimeLists").join(filename);
    let content = if local_path.exists() {
        fs::read_to_string(&local_path)?
    } else {
        let content = reqwest::blocking::get(&*SCUDLEE_MASTER)?.text()?;
        fs::create_dir_all("AnimeLists")?;
        fs::write(&local_path, &content)?;
        content
    };
    let root = Element::parse(content.as_bytes())?;
    // Return all <anime> elements.
    Ok(root.children.iter()
       .filter(|child| child.name == "anime")
       .cloned()
       .collect())
}

/// Search upward for a local custom mapping file (GetAniDBTVDBMapCustom).
pub fn get_anidb_tvdb_map_custom(_media: &Value, _movie: bool) -> Option<Element> {
    // For simplicity, check in the current directory.
    let path = Path::new(&*SCUDLEE_CUSTOM);
    if path.exists() {
        match fs::read_to_string(path) {
            Ok(content) => match Element::parse(content.as_bytes()) {
                Ok(elem) => {
                    info!("Local custom mapping file loaded: {}", path.display());
                    Some(elem)
                },
                Err(e) => {
                    error!("Failed to parse custom mapping file {}: {}", path.display(), e);
                    None
                }
            },
            Err(e) => {
                error!("Failed to read custom mapping file {}: {}", path.display(), e);
                None
            }
        }
    } else {
        info!("Local custom mapping file not present: {}", *SCUDLEE_CUSTOM);
        None
    }
}

/// Load AniDBMovieSets from the SCUDLEE_MOVIESET URL.
pub fn get_anidb_movie_sets() -> Result<Vec<Element>> {
    let filename = Path::new(&*SCUDLEE_MOVIESET)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid SCUDLEE_MOVIESET URL"))?;
    let local_path = Path::new("AnimeLists").join(filename);
    let content = if local_path.exists() {
        fs::read_to_string(&local_path)?
    } else {
        let content = reqwest::blocking::get(&*SCUDLEE_MOVIESET)?.text()?;
        fs::create_dir_all("AnimeLists")?;
        fs::write(&local_path, &content)?;
        content
    };
    let root = Element::parse(content.as_bytes())?;
    if let Some(set_list) = root.get_child("anime-set-list") {
        Ok(set_list.children.clone())
    } else {
        Ok(Vec::new())
    }
}

/// Dummy helper to mimic AniDB.GetAniDBTitle: return (title, main_title, language_rank).
pub fn get_anidb_title<'a>(titles: impl Iterator<Item = &'a Element>) -> (String, String, usize) {
    let title = titles.filter_map(|n| n.get_text()).next().unwrap_or("").to_string();
    (title.clone(), title, 0)
}

/// Dummy helper to mimic common.GetXml.
pub fn get_xml(elem: &Element, child: &str) -> Option<String> {
    elem.get_child(child).and_then(|c| c.get_text()).map(|s| s.to_string())
}

/// Dummy helper to simulate a web link.
pub fn common_web_link(anidb_id: &str) -> Option<String> {
    Some(format!("https://anidb.net/perl-bin/animedb.pl?show=anime&aid={}", anidb_id))
}

/// GetMetadata: Refined version that builds mapping_list and anime_lists_dict from the merged mapping.
/// Returns a tuple:
/// (AnimeLists_dict, AniDB_winner, TVDB_winner, tmdbid, imdbid, mapping_list)
pub fn get_metadata(
    media: &Value,
    movie: bool,
    error_log: &mut Value,
    id: &str,
) -> Result<(Value, String, String, String, String, Value)> {
    info!("{}", "=".repeat(157));
    info!("=== AnimeLists.GetMetadata() ===");
    let mut mapping_list = json!({});
    let mut anime_lists_dict = json!({});
    let mut found = false;

    // Split id into source and value.
    let (source, id_val) = if id.contains('-') {
        let mut parts = id.splitn(2, '-');
        (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
    } else {
        ("", id)
    };
    let mut anidb_id = if source.starts_with("anidb") { id_val } else { "" }.to_string();
    let mut tvdb_id = if source.starts_with("tvdb") { id_val } else { "" }.to_string();
    let tmdb_id = if source.starts_with("tmdb") { id_val } else { "" }.to_string();
    let imdb_id = if source.starts_with("imdb") { id_val } else { "" }.to_string();
    let mut anidb_id2 = String::new();
    let mut tvdb_id2 = String::new();

    // Determine tvdb_numbering from media.seasons.
    let tvdb_numbering = if !movie {
        if let Some(seasons) = media.get("seasons").and_then(|v| v.as_object()) {
            seasons.keys()
                .filter_map(|k| k.parse::<i32>().ok())
                .max()
                .map(|max| max > 1)
                .unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    info!("tvdb_numbering: {}", tvdb_numbering);

    // Load master mapping and merge with custom mapping.
    let master_map = get_anidb_tvdb_map()?;
    let custom_map = get_anidb_tvdb_map_custom(media, movie);
    let anidb_tvdb_map_full = if let Some(custom) = custom_map {
        merge_maps(&master_map, Some(&custom))
    } else {
        master_map
    };

    // Closure to compute anime_core data.
    let anime_core = |anime: &Element| -> (String, String, usize, bool) {
        let defaulttvdbseason = anime.attributes.get("defaulttvdbseason").cloned().unwrap_or_else(|| "1".to_string());
        let episodeoffset = anime.attributes.get("episodeoffset").cloned().unwrap_or_else(|| "0".to_string());
        let s1_mapping_count = anime.get_children("mapping")
            .iter()
            .filter(|m| {
                m.attributes.get("anidbseason").map(|v| v == "1").unwrap_or(false) &&
                m.attributes.get("tvdbseason").map(|v| v == "0" || v == "1").unwrap_or(false)
            })
            .count();
        let s1e1_mapping = anime.get_children("mapping")
            .iter()
            .any(|m| {
                m.attributes.get("anidbseason").map(|v| v == "1").unwrap_or(false) &&
                m.attributes.get("tvdbseason").map(|v| v == "1").unwrap_or(false) &&
                m.get_text().unwrap_or("").contains("-1;")
            });
        let is_primary_series = defaulttvdbseason == "1" && episodeoffset == "0" && (s1_mapping_count == 0 || s1e1_mapping);
        (defaulttvdbseason, episodeoffset, s1_mapping_count, is_primary_series)
    };

    info!("{}", "-".repeat(157));
    info!("--- AniDBTVDBMap ---");
    let forced_id: HashMap<&str, &str> = [("anidbid", &anidb_id[..]), ("tvdbid", &tvdb_id[..]), ("tmdbid", &tmdb_id[..]), ("imdbid", &imdb_id[..])]
        .iter().cloned().collect();

    // Loop through the mapping and try to find a matching anime.
    for anime in &anidb_tvdb_map_full {
        let mut found_id: HashMap<&str, String> = HashMap::new();
        let mut wanted_id: HashMap<&str, bool> = HashMap::new();
        for &key in forced_id.keys() {
            let value = anime.attributes.get(key).cloned().unwrap_or_default();
            found_id.insert(key, value.clone());
            wanted_id.insert(key, !value.is_empty() && value == forced_id[key]);
        }
        if !wanted_id.values().any(|&v| v) {
            continue;
        }
        anidb_id2 = found_id.get("anidbid").cloned().unwrap_or_default();
        tvdb_id2 = found_id.get("tvdbid").cloned().unwrap_or_default();
        if tvdb_id2.is_empty() && anidb_id2.is_empty() {
            continue;
        }
        found = true;
        let (defaulttvdbseason, episodeoffset, s1_mapping_count, is_primary_series) = anime_core(anime);
        if !tvdb_numbering && tvdb_id.is_empty() {
            tvdb_id2 = tvdb_id.clone();
        }
        if tvdb_numbering && !anidb_id2.is_empty() && tvdb_id2.chars().all(|c| c.is_digit(10)) && is_primary_series && anidb_id.is_empty() {
            anidb_id2 = anidb_id2.clone();
        }
        info!("[+] AniDBid: {:>5}, TVDBid: {:>6}, defaulttvdbseason: {:>4}, offset: {:>3}, TMDBid: {:>7}, IMDBid: {:>10}, name: {}",
              anidb_id2,
              tvdb_id2,
              defaulttvdbseason,
              episodeoffset,
              tmdb_id,
              imdb_id,
              anime.get_child("name").and_then(|n| n.get_text()).unwrap_or("")
        );
        // (Additional updates to mapping_list and anime_lists_dict would go here.)
        break; // For demonstration, break after the first match.
    }
    if !found {
        info!("ERROR: Could not find {}: {}", source, id);
        if !anidb_id.is_empty() {
            if let Some(link) = common_web_link(&anidb_id) {
                error!("AniDBid missing: {}", link);
            }
        }
        anidb_id = "".to_string();
        tvdb_id = "".to_string();
    }
    let anidb_winner = if !anidb_id.is_empty() { anidb_id } else { anidb_id2.clone() };
    let tvdb_winner = if tvdb_id2.chars().all(|c| c.is_digit(10)) { tvdb_id2.clone() } else { "".to_string() };

    info!("----- ------");
    info!("{:>5}          {:>6}", anidb_winner, tvdb_winner);

    // Set flags in mapping_list regarding possible anidb3 and s1e1_mapped.
    if source == "tvdb" {
        let mut possible_anidb3 = false;
        if let Some(seasons) = media.get("seasons").and_then(|v| v.as_object()) {
            for (_s, season_val) in seasons {
                if let Some(episodes) = season_val.get("episodes").and_then(|v| v.as_object()) {
                    for (ep, _) in episodes {
                        if ep.parse::<i32>().unwrap_or(0) > 100 {
                            possible_anidb3 = true;
                            break;
                        }
                    }
                }
            }
        }
        set_nested(&mut mapping_list, &["possible_anidb3"], json!(possible_anidb3));
    } else {
        set_nested(&mut mapping_list, &["possible_anidb3"], json!(false));
    }
    let s1e1_mapped = mapping_list.get("TVDB")
        .and_then(|v| v.get("s1"))
        .and_then(|v| v.as_object())
        .map(|obj| obj.values().any(|val| {
            if let Some(arr) = val.as_array() {
                arr.get(0).and_then(|v| v.as_str()) == Some("1")
                    && arr.get(1).and_then(|v| v.as_str()) == Some("1")
            } else {
                false
            }
        }))
        .unwrap_or(false);
    set_nested(&mut mapping_list, &["s1e1_mapped"], json!(s1e1_mapped));

    // Collection and studio update.
    let mut tvdb_collection: Vec<String> = Vec::new();
    let mut title = String::new();
    let mut studio = String::new();
    if !tvdb_winner.is_empty() {
        for anime in &anidb_tvdb_map_full {
            if let Some(tvdb_attr) = anime.attributes.get("tvdbid") {
                if tvdb_attr == &tvdb_winner {
                    tvdb_collection.push(anime.attributes.get("anidbid").cloned().unwrap_or_default());
                    if anime_core(anime).3 {
                        title = anime.get_child("name").and_then(|n| n.get_text()).unwrap_or("").to_string();
                        studio = get_xml(anime, "supplemental-info/studio").unwrap_or_default();
                    }
                }
            }
        }
    }
    set_nested(&mut mapping_list, &["tvdbcount"], json!(tvdb_collection.len()));
    if tvdb_collection.len() > 1 && !title.is_empty() {
        info!("[ ] collection: TVDBid '{}' is part of collection: '{} Collection', related_anime_list: {:?}", tvdb_winner, title, tvdb_collection);
        set_nested(&mut anime_lists_dict, &["collections", "tvdb"], json!([format!("{} Collection", title)]));
    } else {
        info!("[ ] collection: TVDBid '{}' is not part of any collection", tvdb_winner);
    }
    info!("[ ] studio: {}", studio);
    set_nested(&mut anime_lists_dict, &["studio"], json!(studio));

    info!("{}", "-".repeat(157));
    info!("AniDB_id: '{}', AniDB_id2: '{}', TVDB_id: '{}', TVDB_id2: '{}'", anidb_id, anidb_id2, tvdb_id, tvdb_winner);
    info!("mappingList: {:#?}", mapping_list);
    info!("AnimeLists_dict: {:#?}", anime_lists_dict);

    let tmdb_out = mapping_list.get("tmdbid").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let imdb_out = mapping_list.get("imdbid").and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok((anime_lists_dict, anidb_winner, tvdb_winner, tmdb_out, imdb_out, mapping_list))
}

/// Translate AniDB numbering into TVDB numbering.
pub fn tvdb_ep(mapping_list: &Value, season: &str, episode: &str, anidbid: Option<&str>) -> (String, String, String) {
    let ep_part = episode.split('-').next().unwrap_or("");
    let key = format!("s{}e{}", season, ep_part);
    if let Some(val) = get_nested(mapping_list, &["TVDB", &key]) {
        if let Some(arr) = val.as_array() {
            if arr.len() >= 3 {
                return (
                    arr[0].as_str().unwrap_or("").to_string(),
                    arr[1].as_str().unwrap_or("").to_string(),
                    arr[2].as_str().unwrap_or("").to_string(),
                );
            }
        }
    } else if season == "0" {
        return (season.to_string(), episode.to_string(), anidbid.unwrap_or("").to_string());
    } else if season == "1" {
        let default_tvdb_season = mapping_list.get("defaulttvdbseason").and_then(|v| v.as_str()).unwrap_or("1");
        let episodeoffset: i32 = mapping_list.get("episodeoffset").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0);
        let ep_num: i32 = episode.parse().unwrap_or(0);
        return (default_tvdb_season.to_string(), (ep_num + episodeoffset).to_string(), "".to_string());
    }
    (season.to_string(), episode.to_string(), anidbid.unwrap_or("").to_string())
}

/// Translate TVDB numbering into AniDB numbering.
pub fn anidb_ep(mapping_list: &Value, season: &str, episode: &str) -> (String, String, String) {
    let ep_part = episode.split('-').next().unwrap_or("");
    let key = format!("s{}e{}", season, ep_part);
    if let Some(val) = get_nested(mapping_list, &["TVDB", &key]) {
        if let Some(arr) = val.as_array() {
            if arr.len() >= 3 {
                return (
                    arr[0].as_str().unwrap_or("").to_string(),
                    arr[1].as_str().unwrap_or("").to_string(),
                    arr[2].as_str().unwrap_or("").to_string(),
                );
            }
        }
    }
    if season == "1" {
        let episodeoffset: i32 = mapping_list.get("episodeoffset").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0);
        let ep_num: i32 = episode.parse().unwrap_or(0);
        return ("1".to_string(), (ep_num - episodeoffset).to_string(), "".to_string());
    }
    (season.to_string(), episode.to_string(), "xxxxxxx".to_string())
}
