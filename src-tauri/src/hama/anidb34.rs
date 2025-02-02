use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::cmp::Ordering;

/// Returns a vector of keys sorted in (lexicographical) order.
/// (For “natural” sorting you might want to improve this.)
fn sorted_keys(obj: &Value) -> Vec<String> {
    if let Some(map) = obj.as_object() {
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort();
        keys
    } else {
        Vec::new()
    }
}

/// A helper that “saves” a value into a nested JSON object.
/// The keys slice indicates the path. (This is analogous to your SaveDict.)
fn save_dict(mut target: &mut Value, keys: &[&str], value: Value) {
    // Traverse the nested objects creating objects along the way if necessary.
    for key in keys.iter().take(keys.len() - 1) {
        target = target
            .as_object_mut()
            .unwrap()
            .entry(key.to_string())
            .or_insert_with(|| json!({}));
    }
    if let Some(obj) = target.as_object_mut() {
        obj.insert(keys[keys.len() - 1].to_string(), value);
    }
}

/// Adjusts mapping data for AniDB/TVDB as in your Python code.
/// 
/// The parameters are passed as mutable serde_json::Value objects representing dynamic dictionaries:
/// 
/// - **source**: e.g. "tvdb" or "tvdb6"  
/// - **mapping_list**: should contain keys "TVDB", "season_map", "relations_map" and (optionally) "possible_anidb3"
/// - **dict_anidb**  
/// - **dict_thetvdb**  
/// - **dict_fanarttv**
/// 
/// Returns true if modifications were made.
pub fn adjust_mapping(
    source: &str,
    mapping_list: &mut Value,
    dict_anidb: &mut Value,
    dict_thetvdb: &mut Value,
    dict_fanarttv: &mut Value,
) -> bool {
    log::info!("{}", "=".repeat(157));
    log::info!("=== anidb34.AdjustMapping() ===");
    
    let mut is_modified = false;
    let mut adjustments = Map::new();
    let mut tvdb6_seasons: BTreeMap<i64, i64> = BTreeMap::new();
    tvdb6_seasons.insert(1, 1);

    // Get is_banned from dict_anidb["Banned"] (default false)
    let is_banned = dict_anidb
        .get("Banned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Get TVDB, season_map, relations_map from mapping_list (default to empty objects)
    let tvdb = mapping_list
        .get_mut("TVDB")
        .and_then(|v| v.as_object_mut())
        .unwrap_or(&mut Map::new());
    let season_map = mapping_list
        .get("season_map")
        .and_then(|v| v.as_object())
        .unwrap_or(&Map::new());
    let relations_map = mapping_list
        .get("relations_map")
        .and_then(|v| v.as_object())
        .unwrap_or(&Map::new());

    // If not possible_anidb3 present and source is not "tvdb6", log and return
    if mapping_list.get("possible_anidb3").is_none() && source != "tvdb6" {
        log::info!("Neither a possible 'anidb3/tvdb' entry nor 'anidb4/tvdb6' entry");
        return is_modified;
    }
    log::info!("adjusting mapping for 'anidb3/tvdb' & 'anidb4/tvdb6' usage");
    log::info!("season_map: {:?}", season_map);
    log::info!("relations_map: {:?}", relations_map);

    // Wrap the main block in a closure so we can handle errors uniformly.
    let res: Result<()> = (|| {
        log::info!("{}", "-".repeat(157));
        log::info!("--- tvdb mapping adjustments ---");
        log::info!("TVDB Before: {:?}", tvdb);
        // For each id in season_map (sorted by key)
        let mut ids: Vec<&String> = season_map.keys().collect();
        ids.sort(); // simple sort
        for id in ids {
            if id == "max_season" {
                continue;
            }
            let mut new_season = String::new();
            let mut new_episode = String::new();
            log::info!("Checking AniDBid: {}", id);
            // Define a closure to recursively get prequel info.
            let get_prequel_info = |prequel_id: &str| -> Option<(String, String)> {
                // Look up season_map[prequel_id]['min'] and ['max']
                let prequel_entry = season_map.get(prequel_id)?.as_object()?;
                let prequel_min = prequel_entry.get("min")?.as_i64()?;
                let prequel_max = prequel_entry.get("max")?.as_i64()?;
                log::info!(
                    "-- get_prequel_info(prequel_id): {}, season min: {}, season max: {}",
                    prequel_id,
                    prequel_min,
                    prequel_max
                );
                if source == "tvdb" {
                    if prequel_min == 0 {
                        if let Some(rel) = relations_map.get(prequel_id) {
                            if let Some(prequel_arr) = rel.get("Prequel").and_then(|v| v.as_array()) {
                                if let Some(first_prequel) = prequel_arr.get(0).and_then(|v| v.as_str())
                                {
                                    if season_map.contains_key(first_prequel) {
                                        if let Some((a, b)) = get_prequel_info(first_prequel) {
                                            if a.chars().all(|c| c.is_digit(10)) {
                                                let a_int: i64 = a.parse().unwrap_or(0);
                                                let max_season: i64 = season_map
                                                    .get("max_season")
                                                    .and_then(|v| v.as_str())
                                                    .and_then(|s| s.parse().ok())
                                                    .unwrap_or(0);
                                                if a_int < max_season {
                                                    return Some((a, (b.parse::<i64>().unwrap_or(0) + 100).to_string()));
                                                } else {
                                                    return Some(((a.parse::<i64>().unwrap_or(0) + 1).to_string(), "0".to_string()));
                                                }
                                            } else {
                                                return None;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if prequel_min == 0 {
                        return Some((String::new(), String::new()));
                    } else if prequel_max < season_map.get("max_season")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<i64>().ok()).unwrap_or(0)
                    {
                        return Some((prequel_max.to_string(), "100".to_string()));
                    } else {
                        let max_season: i64 = season_map.get("max_season")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                        return Some(((max_season + 1).to_string(), "0".to_string()));
                    }
                }
                if source == "tvdb6" {
                    if prequel_min != 1 {
                        if let Some(rel) = relations_map.get(prequel_id) {
                            if let Some(prequel_arr) = rel.get("Prequel").and_then(|v| v.as_array()) {
                                if let Some(first_prequel) = prequel_arr.get(0).and_then(|v| v.as_str())
                                {
                                    if season_map.contains_key(first_prequel) {
                                        if let Some((a, _b)) = get_prequel_info(first_prequel) {
                                            if a.chars().all(|c| c.is_digit(10)) {
                                                return Some((
                                                    (a.parse::<i64>().unwrap_or(0) + 1 + prequel_max - prequel_min)
                                                        .to_string(),
                                                    "0".to_string(),
                                                ));
                                            } else {
                                                return None;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if prequel_min == 1 {
                        return Some(("2".to_string(), "0".to_string()));
                    } else {
                        return Some((String::new(), String::new()));
                    }
                }
                None
            };

            if source == "tvdb" {
                if let Some(entry) = season_map.get(id) {
                    if entry.get("min").and_then(|v| v.as_i64()) == Some(0) {
                        if let Some(rel) = relations_map.get(id) {
                            if let Some(arr) = rel.get("Prequel").and_then(|v| v.as_array()) {
                                if let Some(first_prequel) = arr.get(0).and_then(|v| v.as_str()) {
                                    if season_map.contains_key(first_prequel) {
                                        if let Some((a, b)) = get_prequel_info(first_prequel) {
                                            new_season = a;
                                            new_episode = b;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if source == "tvdb6" {
                if let Some(rel) = relations_map.get(id) {
                    if let Some(arr) = rel.get("Prequel").and_then(|v| v.as_array()) {
                        if let Some(first_prequel) = arr.get(0).and_then(|v| v.as_str()) {
                            if season_map.contains_key(first_prequel) {
                                if let Some((a, b)) = get_prequel_info(first_prequel) {
                                    new_season = a;
                                    new_episode = b;
                                }
                            }
                        }
                    }
                }
            }

            if !new_season.is_empty() && new_season.chars().all(|c| c.is_digit(10)) {
                is_modified = true;
                let mut removed = Map::new();
                // Iterate over keys in TVDB (which is a mutable Map)
                let tvdb_keys: Vec<String> = tvdb.keys().cloned().collect();
                for key in tvdb_keys {
                    if let Some(val) = tvdb.get_mut(&key) {
                        // If value is an object and contains the current id, remove it.
                        if val.is_object() && val.get(id).is_some() {
                            log::info!("-- Deleted: {}: {{'{}': '{:?}'}}", key, id, val.get(id));
                            let mut temp = Map::new();
                            temp.insert(id.to_string(), val.get(id).unwrap().clone());
                            removed.insert(key.clone(), Value::Object(temp));
                            val.as_object_mut().unwrap().remove(id);
                        }
                        // If value is an array (representing a tuple), check its elements.
                        if val.is_array() {
                            if let Some(arr) = val.as_array() {
                                if arr.get(0).and_then(|v| v.as_str()) == Some("1")
                                    && arr.get(2).and_then(|v| v.as_str()) == Some(id)
                                {
                                    log::info!("-- Deleted: {}: {:?}", key, val);
                                    removed.insert(key.clone(), val.clone());
                                    tvdb.remove(&key);
                                }
                            }
                        }
                    }
                }
                // Save the new episode mapping: TVDB["s{new_season}"][id] = new_episode.
                let season_key = format!("s{}", new_season);
                let new_ep_val = Value::String(new_episode.clone());
                if let Some(season_obj) = tvdb.get_mut(&season_key) {
                    if season_obj.is_object() {
                        season_obj.as_object_mut().unwrap().insert(id.to_string(), new_ep_val);
                    }
                } else {
                    let mut new_map = Map::new();
                    new_map.insert(id.to_string(), new_ep_val);
                    tvdb.insert(season_key.clone(), Value::Object(new_map));
                }
                log::info!("-- Added  : {}: {{'{}': '{}'}}", season_key, id, new_episode);
                let adjust_key = format!("s{}e{}", new_season, new_episode);
                adjustments.insert(
                    adjust_key,
                    json!({
                        "deleted": removed,
                        "added": [new_season.clone(), new_episode.clone()]
                    }),
                );
                // Update tvdb6_seasons[new_season] = season_map[id]["min"]
                if let Some(entry) = season_map.get(id) {
                    if let Some(min_val) = entry.get("min").and_then(|v| v.as_i64()) {
                        if let Ok(new_season_int) = new_season.parse::<i64>() {
                            tvdb6_seasons.insert(new_season_int, min_val);
                        }
                    }
                }
            }
        }
        log::info!("TVDB After : {:?}", tvdb);

        if source == "tvdb6" {
            log::info!("{}", "-".repeat(157));
            log::info!("--- tvdb meta season adjustments ---");
            // Determine top_season from dict_thetvdb["seasons"]
            let top_season = dict_thetvdb.get("seasons")
                .and_then(|v| v.as_object())
                .map(|m| {
                    m.keys()
                        .filter_map(|k| k.parse::<i64>().ok())
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            let mut season = 1;
            let mut adjustment = 0;
            let mut new_seasons_tvdb = Map::new();
            let mut new_seasons_fan = Map::new();
            log::info!("dict_TheTVDB Seasons Before : {:?}", sorted_keys(&dict_thetvdb.get("seasons").unwrap_or(&Value::Null)));
            log::info!("dict_FanartTV Seasons Before : {:?}", sorted_keys(&dict_fanarttv.get("seasons").unwrap_or(&Value::Null)));
            log::info!("tvdb6_seasons : {:?}", tvdb6_seasons);
            if let Some(seasons_obj) = dict_thetvdb.get_mut("seasons").and_then(|v| v.as_object_mut()) {
                if seasons_obj.contains_key("0") {
                    new_seasons_tvdb.insert("0".to_string(), seasons_obj.remove("0").unwrap());
                }
            }
            if let Some(seasons_obj) = dict_fanarttv.get_mut("seasons").and_then(|v| v.as_object_mut()) {
                if seasons_obj.contains_key("0") {
                    new_seasons_fan.insert("0".to_string(), seasons_obj.remove("0").unwrap());
                }
            }
            while season <= top_season {
                let season_plus = season + adjustment;
                if let Some(val) = tvdb6_seasons.get(&season_plus) {
                    if *val == 0 {
                        log::info!("-- New TVDB season  '{}'", season_plus);
                        adjustment += 1;
                    } else {
                        log::info!("-- Adjusting season '{}' -> '{}'", season, season_plus);
                        if let Some(seasons_obj) = dict_thetvdb.get_mut("seasons").and_then(|v| v.as_object_mut()) {
                            if seasons_obj.contains_key(&season.to_string()) {
                                new_seasons_tvdb.insert((season_plus).to_string(), seasons_obj.remove(&season.to_string()).unwrap());
                            }
                        }
                        if let Some(seasons_obj) = dict_fanarttv.get_mut("seasons").and_then(|v| v.as_object_mut()) {
                            if seasons_obj.contains_key(&season.to_string()) {
                                new_seasons_fan.insert((season_plus).to_string(), seasons_obj.remove(&season.to_string()).unwrap());
                            }
                        }
                        season += 1;
                    }
                } else {
                    season += 1;
                }
            }
            dict_thetvdb.as_object_mut().unwrap().insert("seasons".to_string(), Value::Object(new_seasons_tvdb));
            dict_fanarttv.as_object_mut().unwrap().insert("seasons".to_string(), Value::Object(new_seasons_fan));
            log::info!("dict_TheTVDB Seasons After : {:?}", sorted_keys(dict_thetvdb.get("seasons").unwrap()));
            log::info!("dict_FanartTV Seasons After : {:?}", sorted_keys(dict_fanarttv.get("seasons").unwrap()));
        }
        
        // tvdb meta episode adjustments
        log::info!("{}", "-".repeat(157));
        log::info!("--- tvdb meta episode adjustments ---");
        log::info!("adjustments: {:?}", adjustments);
        for entry in sorted_keys(&Value::Object(adjustments.clone())) {
            if let Some(added) = adjustments.get(&entry) {
                let added_season = added.get("added")
                    .and_then(|arr| arr.get(0))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let added_offset = added.get("added")
                    .and_then(|arr| arr.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                log::info!("added_season: '{}', added_offset: '{}'", added_season, added_offset);
                if let Some(deleted_obj) = added.get("deleted").and_then(|v| v.as_object()) {
                    for deleted in sorted_keys(&Value::Object(deleted_obj.clone())) {
                        let deleted_value = deleted_obj.get(&deleted).unwrap();
                        if deleted_value.is_object() {
                            let deleted_season = deleted.trim_start_matches('s');
                            let deleted_offset = deleted_value.as_object().unwrap()
                                .values()
                                .next()
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if deleted == "s-1" {
                                log::info!("---- '{}': Dead season", deleted);
                                continue;
                            }
                            if deleted != "s0" && added_offset == "0" && deleted_offset == "0" {
                                log::info!("---- '{}': Whole season (s1+) was adjusted in previous section", deleted);
                                continue;
                            }
                            let mut iteration = 1;
                            log::info!("---- deleted_season: '{}', deleted_offset: '{}'", deleted_season, deleted_offset);
                            loop {
                                let key_val = (deleted_offset.parse::<i32>().unwrap_or(0) + iteration).to_string();
                                // Check if dict_thetvdb["seasons"][deleted_season]["episodes"] has key key_val.
                                let exists = dict_thetvdb.get("seasons")
                                    .and_then(|v| v.as_object())
                                    .and_then(|m| m.get(deleted_season))
                                    .and_then(|v| v.get("episodes"))
                                    .and_then(|v| v.as_object())
                                    .and_then(|m| m.get(&key_val))
                                    .is_some();
                                if !exists {
                                    break;
                                }
                                let a = deleted_season;
                                let b = key_val.clone();
                                let x = (added_offset.parse::<i32>().unwrap_or(0) + iteration).to_string();
                                // Copy the value from dict_thetvdb["seasons"][a]["episodes"][b] into dict_thetvdb["seasons"][added_season]["episodes"][x]
                                if let Some(source_val) = dict_thetvdb.get("seasons")
                                    .and_then(|v| v.as_object())
                                    .and_then(|m| m.get(a))
                                    .and_then(|v| v.get("episodes"))
                                    .and_then(|v| v.as_object())
                                    .and_then(|m| m.get(&b))
                                    .cloned()
                                {
                                    if let Some(season_obj) = dict_thetvdb.get_mut("seasons")
                                        .and_then(|v| v.as_object_mut())
                                        .and_then(|m| m.get_mut(added_season))
                                        .and_then(|v| v.as_object_mut())
                                    {
                                        if let Some(eps_obj) = season_obj.get_mut("episodes")
                                            .and_then(|v| v.as_object_mut())
                                        {
                                            eps_obj.insert(x.clone(), source_val);
                                        }
                                    }
                                    log::info!("---- '{}': dict_TheTVDB['seasons']['{}']['episodes']['{}'] => dict_TheTVDB['seasons']['{}']['episodes']['{}']",
                                        deleted, a, b, added_season, x);
                                }
                                iteration += 1;
                            }
                        }
                        if deleted_value.is_array() {
                            let parts: Vec<&str> = deleted
                                .chars()
                                .filter(|c| *c == 's' || *c == 'e')
                                .collect::<Vec<char>>(); // (this is a simplification)
                            // For simplicity, we split the string by 's' and 'e'
                            let parts: Vec<&str> = deleted.split(|c| c == 's' || c == 'e').filter(|s| !s.is_empty()).collect();
                            if parts.len() >= 2 {
                                let a = parts[0];
                                let b = parts[1];
                                if let Some(arr) = deleted_value.as_array() {
                                    let second = arr.get(1)
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("0")
                                        .parse::<i32>()
                                        .unwrap_or(0);
                                    let x = (second + added_offset.parse::<i32>().unwrap_or(0)).to_string();
                                    log::info!("---- '{}': dict_TheTVDB['seasons']['{}']['episodes']['{}'] => dict_TheTVDB['seasons']['{}']['episodes']['{}']",
                                        deleted, a, b, added_season, x);
                                    if let Some(source_val) = dict_thetvdb.get("seasons")
                                        .and_then(|v| v.as_object())
                                        .and_then(|m| m.get(a))
                                        .and_then(|v| v.get("episodes"))
                                        .and_then(|v| v.as_object())
                                        .and_then(|m| m.get(b))
                                        .cloned()
                                    {
                                        if let Some(season_obj) = dict_thetvdb.get_mut("seasons")
                                            .and_then(|v| v.as_object_mut())
                                            .and_then(|m| m.get_mut(added_season))
                                            .and_then(|v| v.as_object_mut())
                                        {
                                            if let Some(eps_obj) = season_obj.get_mut("episodes")
                                                .and_then(|v| v.as_object_mut())
                                            {
                                                eps_obj.insert(x.clone(), source_val);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })();
    
    if let Err(e) = res {
        if is_banned {
            log::info!("Expected exception hit as you were banned from AniDB so you have incomplete data to proceed");
        } else {
            log::error!("Unexpected exception hit: {}", e);
        }
        log::info!("If a key error, look at the 'season_map'/'relations_map' info to see why it is missing");
        if source == "tvdb" {
            log::info!("Source is 'tvdb' so metadata will be loaded but it will not be complete for any 'anidb3' end of season additions");
        }
        if source == "tvdb6" {
            log::info!("Source is 'tvdb6' so removing AniDB & TVDB metadata from memory to prevent incorrect data from being loaded");
            dict_anidb.as_object_mut().map(|o| o.clear());
            dict_thetvdb.as_object_mut().map(|o| o.clear());
        }
        return false;
    }
    
    log::info!("{}", "-".repeat(157));
    log::info!("--- return ---");
    log::info!("is_modified: {}", is_modified);
    is_modified
}
