use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

const DEFAULT_TOKEN: &str = env!("ANNICT_TOKEN");
const BASE_URL: &str = "https://api.annict.com/v1";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnnictWork {
    id: i32,
    title: String,
    media_text: Option<String>,
    season_name_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnnictEpisode {
    id: i32,
    number: Option<f32>,
    sort_number: Option<i32>,
    number_text: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameTask {
    old_path: String,
    new_path: String,
    old_name: String,
    new_name: String,
}

#[tauri::command]
async fn search_works(query: String) -> Result<Vec<AnnictWork>, String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", DEFAULT_TOKEN)).unwrap(),
    );

    let url = format!("{}/works", BASE_URL);
    let resp = client
        .get(url)
        .headers(headers)
        .query(&[("filter_title", &query), ("sort_id", &"desc".to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let works: Vec<AnnictWork> = serde_json::from_value(data["works"].clone()).map_err(|e| e.to_string())?;

    Ok(works)
}

#[tauri::command]
async fn get_episodes(work_id: i32) -> Result<Vec<AnnictEpisode>, String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", DEFAULT_TOKEN)).unwrap(),
    );

    let mut episodes = Vec::new();
    let mut page = 1;

    loop {
        let url = format!("{}/episodes", BASE_URL);
        let resp = client
            .get(url)
            .headers(headers.clone())
            .query(&[
                ("filter_work_id", &work_id.to_string()),
                ("sort_id", &"asc".to_string()),
                ("page", &page.to_string()),
                ("per_page", &"50".to_string()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let eps: Vec<AnnictEpisode> =
            serde_json::from_value(data["episodes"].clone()).map_err(|e| e.to_string())?;

        if eps.is_empty() {
            break;
        }

        episodes.extend(eps);

        let total_count = data["total_count"].as_u64().unwrap_or(0) as usize;
        if episodes.len() >= total_count {
            break;
        }
        page += 1;
    }

    Ok(episodes)
}

#[tauri::command]
fn get_rename_tasks(path: String, episodes: Vec<AnnictEpisode>) -> Result<Vec<RenameTask>, String> {
    let target_path = PathBuf::from(&path);
    if !target_path.exists() {
        return Err("Path does not exist".into());
    }

    let exts = ["mp4", "mkv", "avi", "mov", "wmv"];
    let mut files: Vec<PathBuf> = if target_path.is_file() {
        vec![target_path]
    } else {
        fs::read_dir(&target_path)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| exts.contains(&s.to_lowercase().as_str()))
                        .unwrap_or(false)
            })
            .collect()
    };

    // Natural sort for directory listing
    if files.len() > 1 {
        files.sort_by(|a, b| {
            natord::compare(
                a.file_name().unwrap().to_str().unwrap_or(""),
                b.file_name().unwrap().to_str().unwrap_or(""),
            )
        });
    }

    let mut tasks = Vec::new();
    let ep_map = build_episode_index(&episodes);

    for file_path in files {
        let filename = file_path.file_name().unwrap().to_str().unwrap();
        if let Some(num) = extract_episode_number(filename, &ep_map) {
            if let Some(ep) = ep_map.get(&num) {
                let title_part = if let Some(t) = &ep.title {
                    if t.is_empty() { "".to_string() } else { format!(" - {}", t) }
                } else {
                    "".to_string()
                };
                
                let ext = file_path.extension().unwrap().to_str().unwrap();
                let new_name = sanitize_filename(&format!("{:0>2}{}.{}", num, title_part, ext));
                let new_path = file_path.with_file_name(&new_name);

                tasks.push(RenameTask {
                    old_path: file_path.to_str().unwrap().to_string(),
                    new_path: new_path.to_str().unwrap().to_string(),
                    old_name: filename.to_string(),
                    new_name,
                });
            }
        }
    }

    Ok(tasks)
}

#[tauri::command]
fn execute_renames(tasks: Vec<RenameTask>) -> Result<usize, String> {
    let mut success = 0;
    for task in tasks {
        let old = PathBuf::from(&task.old_path);
        let new = PathBuf::from(&task.new_path);

        if new.exists() && old != new {
            continue;
        }

        if let Err(e) = fs::rename(old, new) {
            println!("Error renaming: {}", e);
        } else {
            success += 1;
        }
    }
    Ok(success)
}

fn build_episode_index(episodes: &[AnnictEpisode]) -> std::collections::HashMap<String, AnnictEpisode> {
    let mut map = std::collections::HashMap::new();
    for ep in episodes {
        if let Some(num) = ep.number {
            map.insert((num as i32).to_string(), ep.clone());
        }
        if let Some(sort_num) = ep.sort_number {
            let key = (sort_num / 10).to_string();
            if !map.contains_key(&key) {
                map.insert(key, ep.clone());
            }
        }
    }
    map
}

fn extract_episode_number(filename: &str, ep_map: &std::collections::HashMap<String, AnnictEpisode>) -> Option<String> {
    let stem = std::path::Path::new(filename).file_stem()?.to_str()?;
    let mut name = stem.to_string();

    // 1. Preprocessing (CRC removal)
    let crc_re = Regex::new(r"\[[0-9a-fA-F]{8}\]").unwrap();
    name = crc_re.replace_all(&name, "").to_string();

    // Noise removal
    let noise = [
        r"(?i)(?:\d{3,4}p?|(?:10|8)bit|x26[45]|h26[45]|hevc|av1|bdrip|web-dl|tvrip)",
        r"(?:20[0-2][0-9]|19[8-9][0-9])",
    ];
    for p in noise {
        let re = Regex::new(p).unwrap();
        name = re.replace_all(&name, " ").to_string();
    }

    // 2. Pattern matching
    // High priority tags
    let tags = [
        r"(?i)ep(?:isode)?\.?\s*(\d{1,3})(?:v\d+)?",
        r"第\s*(\d{1,3})(?:v\d+)?\s*[話话]",
        r"#\s*(\d{1,3})(?:v\d+)?",
    ];
    for p in tags {
        let re = Regex::new(p).unwrap();
        if let Some(cap) = re.captures(&name) {
            let num = cap.get(1).unwrap().as_str().parse::<i32>().unwrap_or(-1).to_string();
            if ep_map.contains_key(&num) {
                return Some(num);
            }
        }
    }

    // Delimiters
    let delims = [
        r"[\s\-\_\(\)\[\]](\d{1,3})(?:v\d+)?[\s\-\_\(\)\[\]]",
        r"^\s*(\d{1,3})(?:v\d+)?[\s\-\_\(\)\[\]]",
        r"[\s\-\_\(\)\[\]](\d{1,3})(?:v\d+)?\s*$",
    ];
    for p in delims {
        let re = Regex::new(p).unwrap();
        let matches: Vec<_> = re.captures_iter(&name).collect();
        for cap in matches.iter().rev() {
            let num = cap.get(1).unwrap().as_str().parse::<i32>().unwrap_or(-1).to_string();
            if ep_map.contains_key(&num) {
                return Some(num);
            }
        }
    }

    // Last resort
    let pot_re = Regex::new(r"(\d{1,3})(?:v\d+)?").unwrap();
    let potentials: Vec<_> = pot_re.captures_iter(&name).collect();
    for cap in potentials.iter().rev() {
        let num = cap.get(1).unwrap().as_str().parse::<i32>().unwrap_or(-1).to_string();
        if ep_map.contains_key(&num) {
            return Some(num);
        }
    }

    None
}

fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[\\/:*?"<>|]"#).unwrap();
    re.replace_all(name, "_").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            search_works,
            get_episodes,
            get_rename_tasks,
            execute_renames
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
