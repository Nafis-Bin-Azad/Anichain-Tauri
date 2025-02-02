use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use roxmltree;
use std::io::Read;
use std::time::Instant;
use strsim::levenshtein;

/// A simplified media structure (either a movie or series)
#[derive(Debug, Clone)]
pub struct Media {
    /// For movies, this is the title; for TV series, this is the “show” name.
    pub title: String,
    /// For TV series you might have a separate “show” name.
    pub show: String,
    /// Optionally, the release year.
    pub year: Option<i32>,
}

/// A search result from AniDB
#[derive(Debug, Clone)]
pub struct MetadataSearchResult {
    /// The AniDB id (prefixed with “anidb-”)
    pub id: String,
    /// A human‑readable title (with additional info)
    pub name: String,
    /// Year (if known)
    pub year: Option<i32>,
    /// Language (for now, a string)
    pub lang: String,
    /// Score (a number from 0 to 100)
    pub score: f32,
}

/// A simple container for the parsed AniDB titles XML document.
pub struct AniDBTitlesDB {
    pub doc: roxmltree::Document<'static>,
}

/// Download and load the AniDB titles XML (a gzipped file) from AniDB’s site.
/// (In the Python code this is cached only every two weeks.)
pub async fn get_anidb_titles_db() -> Result<AniDBTitlesDB> {
    const ANIDB_TITLES_URL: &str = "https://anidb.net/api/anime-titles.xml.gz";
    let client = reqwest::Client::new();
    let response = client.get(ANIDB_TITLES_URL).send().await?.bytes().await?;
    let mut decoder = GzDecoder::new(&response[..]);
    let mut s = String::new();
    decoder.read_to_string(&mut s)?;
    // We use roxmltree to parse the XML
    let doc = roxmltree::Document::parse(&s)
        .map_err(|e| anyhow!("XML parse error: {}", e))?;
    Ok(AniDBTitlesDB { doc })
}

/// A very basic title‐cleansing function. (You can expand this to mimic your Python common.cleanse_title.)
pub fn cleanse_title(title: &str) -> String {
    title.to_lowercase().trim().to_string()
}

/// Returns the length of the longest common substring of two strings.
fn longest_common_substring(a: &str, b: &str) -> usize {
    let m = a.len();
    let n = b.len();
    let mut result = 0;
    // Create a 2D table with dimensions (m+1) x (n+1)
    let mut table = vec![vec![0; n + 1]; m + 1];
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    for i in 0..m {
        for j in 0..n {
            if a_chars[i] == b_chars[j] {
                table[i + 1][j + 1] = table[i][j] + 1;
                if table[i + 1][j + 1] > result {
                    result = table[i + 1][j + 1];
                }
            }
        }
    }
    result
}

/// Computes a “words score” comparing an array of words to a cleansed title string.
/// (This is similar to your Python’s WordsScore function.)
pub fn words_score(words: &[&str], title_cleansed: &str) -> f32 {
    let max_length = words.join("").len().max(title_cleansed.len());
    if max_length == 0 {
        return 0.0;
    }
    let mut score = 0.0;
    for word in words {
        let lcs = longest_common_substring(word, title_cleansed) as f32;
        score += 100.0 * lcs / (max_length as f32);
    }
    score
}

/// Performs an AniDB search given a media object and returns a best‑score plus a list of results.
/// (This function roughly corresponds to your Python Search function.)
pub async fn anidb_search(
    media: &Media,
    lang: &str,
    manual: bool,
    movie: bool,
    titles_db: &AniDBTitlesDB,
) -> Result<(f32, i32, Vec<MetadataSearchResult>)> {
    // These are words that we want to ignore when doing keyword matching.
    let filter_search_words = vec![
        "to", "wa", "ga", "no", "age", "da", "chou", "super", "yo", "de", "chan", "hime", "ni", "sekai",
        "a", "of", "an", "the", "motion", "picture", "special", "oav", "ova", "tv", "special", "eternal", "final", "last", "one", "movie", "me", "princess", "theater", "and",
        "le", "la", "un", "les", "nos", "vos", "des", "ses", "world", "in", "another", "this", "story", "life", "name",
        "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv", "xv", "xvi",
    ];
    let split_chars: Vec<char> = vec![';', ':', '*', '?', ',', '.', '~', '-', '\\', '/'];

    // Use media.title for movies; for series use media.show.
    let orig_title = if movie {
        media.title.clone()
    } else {
        media.show.clone()
    };
    let orig_title_cleansed = cleanse_title(&orig_title);
    println!("orig_title: '{}', cleansed: '{}'", orig_title, orig_title_cleansed);

    let mut results: Vec<MetadataSearchResult> = Vec::new();
    let mut best_aid = String::new();
    let mut best_score = 0.0;
    let mut best_title = String::new();
    let mut n = 0;
    let start_time = Instant::now();

    // Full title search (simulate an XPath search by iterating over <title> nodes)
    for node in titles_db.doc.descendants().filter(|n| n.has_tag_name("title")) {
        if let Some(text) = node.text() {
            if text.to_lowercase().contains(&orig_title.to_lowercase()) {
                let aid = node.parent().and_then(|p| p.attribute("aid")).unwrap_or("");
                let title = text;
                let title_cleansed = cleanse_title(title);
                let score = if orig_title == title {
                    100.0
                } else if orig_title.to_lowercase() == title.to_lowercase() {
                    99.0
                } else {
                    // Use longest common substring and Levenshtein distance as a proxy for similarity.
                    let lcs_len = longest_common_substring(&orig_title_cleansed, &title_cleansed);
                    let score1 = 100.0 * (lcs_len as f32)
                        / (title_cleansed.len().max(orig_title_cleansed.len()) as f32)
                        - n as f32;
                    let lev = levenshtein(&orig_title_cleansed, &title_cleansed) as f32;
                    let score2 = 100.0 - 100.0 * lev
                        / (title_cleansed.len().max(orig_title_cleansed.len()) as f32)
                        - n as f32;
                    score1.max(score2)
                };
                if score >= 100.0 && aid == best_aid {
                    continue;
                }
                if score >= 100.0 {
                    n += 1;
                }
                results.push(MetadataSearchResult {
                    id: format!("anidb-{}", aid),
                    name: format!("{} [anidb-{}]", title, aid),
                    year: media.year,
                    lang: lang.to_string(),
                    score,
                });
                if score > best_score {
                    best_score = score;
                    best_title = title.to_string();
                    best_aid = aid.to_string();
                }
            }
        }
    }

    println!(
        "[=] best_score: {}, best_aid: {}, best_title: {}",
        best_score, best_aid, best_title
    );
    println!("Elapsed time: {:?}", start_time.elapsed());
    if best_score >= 90.0 {
        return Ok((best_score, n, results));
    }

    // If full title search did not yield a high score, perform a keyword search.
    let mut cleansed = orig_title_cleansed.clone();
    for c in &split_chars {
        cleansed = cleansed.replace(*c, " ");
    }
    cleansed = cleansed.replace("'", "");
    let mut words: Vec<&str> = Vec::new();
    let mut words_skipped: Vec<&str> = Vec::new();
    for word in cleansed.split_whitespace() {
        if filter_search_words.contains(&word) || word.len() <= 3 {
            words_skipped.push(word);
        } else {
            words.push(word);
        }
    }
    if words.is_empty() {
        words = cleansed.split_whitespace().collect();
        words_skipped.clear();
    }
    println!(
        "Keyword Search - Words: {:?}, skipped: {:?}",
        words, words_skipped
    );
    let mut best_score_keyword = 0.0;
    let mut best_title_keyword = String::new();
    let mut best_aid_keyword = String::new();
    let mut last_chance = Vec::new();

    // Iterate over each <anime> node
    for anime in titles_db.doc.descendants().filter(|n| n.has_tag_name("anime")) {
        let aid = anime.attribute("aid").unwrap_or("");
        let mut best_score_entry = 0.0;
        let mut best_title_entry = String::new();
        for title_node in anime.children().filter(|n| n.has_tag_name("title")) {
            if let Some(text) = title_node.text() {
                let text_lower = text.to_lowercase();
                if words.iter().all(|w| text_lower.contains(*w)) {
                    let title_cleansed = cleanse_title(text);
                    let score = if title_cleansed == orig_title_cleansed {
                        if text.contains(";") {
                            98.0
                        } else {
                            100.0
                        }
                    } else {
                        words_score(&words, &title_cleansed)
                    };
                    if score > best_score_entry {
                        best_score_entry = score;
                        best_title_entry = text.to_string();
                    }
                }
            }
        }
        if best_score_entry < 25.0 {
            last_chance.push((best_score_entry, best_title_entry.clone(), aid.to_string()));
            continue;
        }
        results.push(MetadataSearchResult {
            id: format!("anidb-{}", aid),
            name: format!("{} [keyword]", best_title_entry),
            year: media.year,
            lang: lang.to_string(),
            score: best_score_entry,
        });
        if best_score_entry > best_score_keyword {
            best_score_keyword = best_score_entry;
            best_title_keyword = best_title_entry;
            best_aid_keyword = aid.to_string();
        }
    }
    if best_score_keyword < 50.0 {
        for (score, title, aid) in last_chance {
            results.push(MetadataSearchResult {
                id: format!("anidb-{}", aid),
                name: format!("{} [keyword fallback]", title),
                year: media.year,
                lang: lang.to_string(),
                score,
            });
            if score > best_score_keyword {
                best_score_keyword = score;
                best_title_keyword = title;
                best_aid_keyword = aid;
            }
        }
    }
    println!("Elapsed time (keyword search): {:?}", start_time.elapsed());
    Ok((best_score_keyword, n, results))
}

/// Sanitizes an AniDB summary string by cleaning up unwanted links and extra lines.
pub fn summary_sanitizer(summary: &str) -> String {
    let mut s = summary.replace("`", "'");
    let re_link = Regex::new(r"https?://anidb\.net/[a-z]{1,2}[0-9]+ \[(?P<text>.+?)\]").unwrap();
    s = re_link.replace_all(&s, "$text").to_string();
    let re_link_long = Regex::new(r"https?://anidb\.net/[a-z]+/[0-9]+ \[(?P<text>.+?)\]").unwrap();
    s = re_link_long.replace_all(&s, "$text").to_string();
    let re_line = Regex::new(r"^(?:\*|--|~) .*").unwrap();
    s = re_line.replace_all(&s, "").to_string();
    let re_after = Regex::new(r"\n(Source|Note|Summary):.*").unwrap();
    s = re_after.replace_all(&s, "").to_string();
    let re_empty = Regex::new(r"\n\n+").unwrap();
    s = re_empty.replace_all(&s, "\n\n").to_string();
    s.trim().to_string()
}

/// Given an iterator over XML title nodes, choose the best title according to language and type priority.
/// (This is a simplified version of your Python GetAniDBTitle.)
pub fn get_anidb_title<'a>(
    titles: impl Iterator<Item = roxmltree::Node<'a>>,
) -> (String, String, usize) {
    let languages = vec!["english", "japanese", "main"];
    let mut lang_levels: Vec<u32> = vec![20; languages.len()];
    let mut lang_titles: Vec<String> = vec![String::new(); languages.len()];
    for title in titles {
        let lang = title.attribute("xml:lang").unwrap_or("main").to_lowercase();
        let type_attr = title.attribute("type").unwrap_or("");
        let text = title.text().unwrap_or("").replace("`", "'");
        // A simple type priority: lower numbers are higher priority.
        let priority = match type_attr {
            "main" => 1,
            "official" => 2,
            "kana" => 3,
            "card" => 4,
            "syn" => 5,
            "short" => 6,
            _ => 7,
        };
        if let Some(idx) = languages.iter().position(|&l| l == lang) {
            if priority < lang_levels[idx] {
                lang_levels[idx] = priority;
                lang_titles[idx] = text;
            }
        }
    }
    let mut index = 0;
    for (i, title) in lang_titles.iter().enumerate() {
        if !title.is_empty() {
            index = i;
            break;
        }
    }
    let main_index = languages.iter().position(|&x| x == "main").unwrap_or(0);
    let main_title = lang_titles.get(main_index).cloned().unwrap_or_default();
    (lang_titles[index].clone(), main_title, index)
}
