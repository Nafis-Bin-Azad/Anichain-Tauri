use anyhow::{Result, anyhow};
use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, MAIN_SEPARATOR};
use log::{info, debug};

/// Constant season regular expressions (as in your Python code)
static SEASON_RX: [&str; 5] = [
    r"^Specials", // Specials (season 0)
    r"^(Season|Series|Book|Saison|Livre|Temporada|S)[ _\-\.]*(?P<season>\d{1,4})", // Season / Series / Book / Saison / Livre / S
    r"^(?P<show>.*?)[\._\- ]+S(?P<season>\d{2})$", // (title) S01
    r"^(?P<season>\d{1,2})", // simple numeric folder
    r"^(Saga|(Story )?Ar[kc])", // e.g. Saga / Story Ark / Ark
];

/// Dummy implementation of common.GetMediaDir(media, movie).
/// (In your actual application, this should return the media’s directory.)
fn common_get_media_dir(media: &Value, _movie: bool) -> String {
    // For demonstration, assume media is a JSON object with a "dir" field.
    media.get("dir")
         .and_then(|v| v.as_str())
         .unwrap_or("")
         .to_string()
}

/// Dummy implementation of common.GetLibraryRootPath(dir).
/// Returns a tuple (library, root, path) given a directory string.
/// For example, if dir is "/foo/bar/baz", we assume the library is "Library",
/// root is "/foo" and path is "bar/baz".
fn common_get_library_root_path(dir: &str) -> (String, String, String) {
    let p = Path::new(dir);
    if let Some(parent) = p.parent() {
        let library = "Library".to_string();
        let root = parent.to_string_lossy().to_string();
        // Compute relative path from the parent:
        let rel = p.strip_prefix(parent).unwrap_or(p);
        let path = rel.to_string_lossy().to_string();
        (library, root, path)
    } else {
        ("".to_string(), "".to_string(), "".to_string())
    }
}

/// Dummy SaveDict: inserts the given value into the target JSON object under the specified keys.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) {
    // Use our helper set_nested
    set_nested(target, keys, value);
}

/// Set a nested value in a JSON object. Creates intermediate objects if necessary.
fn set_nested(target: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *target = new_value;
        return;
    }
    let mut current = target;
    for key in &keys[..keys.len() - 1] {
        if !current.get(*key).is_some() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len() - 1]] = new_value;
}

/// Dummy DictString: returns a string representation of a JSON value (for logging).
fn dict_string(val: &Value, indent: usize) -> String {
    serde_json::to_string_pretty(val).unwrap_or_else(|_| format!("{:?}", val))
}

/// GetMetadata for Local media.
///
/// Given a media object (as JSON) and a boolean flag indicating whether the media is a movie,
/// this function inspects the directory structure to extract grouping information and
/// (if applicable) a collection name.
///
/// In the Python code the following steps occur:
///  1. Get the media directory and library root.
///  2. If not a movie, and if the relative path isn’t “_unknown_folder” or “.”, then:
///      a. Determine the series root folder from the first path segment.
///      b. Count subdirectories.
///      c. Reverse the path segments and check for season folder names using SEASON_RX.
///      d. If grouping folders are detected, save the last folder’s name into Local_dict["collections"].
///  3. Log and return the resulting Local_dict.
pub fn get_metadata(media: &Value, movie: bool) -> Result<Value> {
    info!("{}", "=== Local.GetMetadata() ===".repeat(1));
    let mut local_dict = json!({});
    let dir = common_get_media_dir(media, movie);
    let (library, root, path) = common_get_library_root_path(&dir);

    // If movie, return an empty dict.
    if movie {
        return Ok(local_dict);
    }

    info!("dir:     {}", dir);
    info!("library: {}", library);
    info!("root:    {}", root);
    info!("path:    {}", path);

    if path != "_unknown_folder" && path != "." && !path.is_empty() {
        // Get the first path segment.
        let path_parts: Vec<&str> = path.split(MAIN_SEPARATOR).collect();
        if path_parts.is_empty() {
            return Ok(local_dict);
        }
        let series_root_folder = Path::new(&root).join(path_parts[0]);
        info!("series_root_folder:  {:?}", series_root_folder);
        let grouping_folder = series_root_folder.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        info!("Grouping folder:     {}", grouping_folder);
        if !series_root_folder.exists() {
            info!("files are currently inaccessible");
            return Ok(local_dict);
        }
        // Count subdirectories in the series_root_folder.
        let subfolder_count = fs::read_dir(&series_root_folder)?
            .filter(|entry| entry.as_ref().map(|e| e.path().is_dir()).unwrap_or(false))
            .count();
        info!("subfolder_count:     {}", subfolder_count);

        // Extract season and transparent folder to reduce complexity.
        // reverse_path: a vector of folder names from the path, in reverse order.
        let mut reverse_path: Vec<&str> = path.split(MAIN_SEPARATOR).collect();
        reverse_path.reverse();
        let mut season_folder_first = false;
        // We iterate over a copy of reverse_path except the last element.
        let copy = reverse_path.clone();
        if copy.len() > 1 {
            for folder in &copy[..copy.len()-1] {
                for rx in &SEASON_RX {
                    let re = Regex::new(&format!("(?i){}", rx)).map_err(|e| anyhow!("Regex error: {}", e))?;
                    if re.is_match(folder) {
                        // Remove the folder from reverse_path.
                        if let Some(pos) = reverse_path.iter().position(|&f| f == *folder) {
                            reverse_path.remove(pos);
                            // If the regex is not the last one in SEASON_RX and there are at least 2 elements left,
                            // and if folder equals the second-to-last element, mark season_folder_first.
                            if *rx != SEASON_RX[SEASON_RX.len()-1] && reverse_path.len() >= 2 &&
                               reverse_path[reverse_path.len()-2] == *folder {
                                season_folder_first = true;
                            }
                        }
                        break;
                    }
                }
            }
        }
        info!("reverse_path:        {:?}", reverse_path);
        info!("season_folder_first: {}", season_folder_first);
        if reverse_path.len() > 1 && !season_folder_first && subfolder_count > 1 {
            // Use the last element of reverse_path as the grouping folder.
            let collection = vec![reverse_path[reverse_path.len()-1].to_string()];
            save_dict(&mut local_dict, &["collections"], json!(collection));
            info!("[ ] collection (Grouping folder): {:?}", local_dict["collections"]);
        } else {
            info!("Grouping folder not found");
        }
    }

    info!("{}", "--- return ---".repeat(1));
    info!("Local_dict: {}", dict_string(&local_dict, 1));
    Ok(local_dict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;
    use std::fs;
    use std::io::Write;

    // Dummy test for get_metadata.
    #[test]
    fn test_get_metadata() {
        // Initialize logger for testing.
        let _ = env_logger::builder().is_test(true).try_init();

        // Create a dummy media JSON.
        // Assume that common_get_media_dir will return the "dir" field.
        let media = json!({
            "dir": format!("{}{}TestSeries{}Season 01", env::temp_dir().display(), MAIN_SEPARATOR, MAIN_SEPARATOR)
        });
        // For the purpose of this test, we create the following directory structure:
        // temp_dir/TestSeries/Season 01/
        let base_dir = env::temp_dir().join("TestSeries");
        let season_dir = base_dir.join("Season 01");
        fs::create_dir_all(&season_dir).unwrap();
        // Also, create a few dummy subdirectories in the series root folder.
        fs::create_dir_all(&base_dir.join("ExtraFolder")).unwrap();

        let result = get_metadata(&media, false).unwrap();
        println!("Result: {}", dict_string(&result, 2));

        // Clean up (in a real test use tempdir crate).
        fs::remove_dir_all(&base_dir).unwrap();
    }
}
