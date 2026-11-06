//! Tolerant libretro thumbnail lookup.
//!
//! libretro names its box arts after exact dump names ("DuckTales (Europe)",
//! "Skate or Die (Europe)", "Wizards _ Warriors III - ..."), so a ROM named
//! "Duck Tales (Europe)" or "Skate or Die! (Europe)" misses every exact URL.
//! This downloads the system's published file list once (cached on disk a
//! week: the NES list is 4 MB), matches on the title alone (case, spaces and
//! punctuation ignored, region/version tags dropped), and picks the closest
//! file: same region, clean dump, no beta/proto.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use crate::push_log;

const INDEX_TTL_SECS: u64 = 7 * 24 * 3600;

type Index = HashMap<String, Vec<String>>;

/// "Skate or Die! (Europe)" → "skateordie"; "Legend of Zelda, The" → "legendofzelda".
pub fn title_key(name: &str) -> String {
    let base = name.split(['(', '[']).next().unwrap_or(name).trim().to_lowercase();
    let base = base
        .strip_prefix("the ")
        .or_else(|| base.strip_prefix("a "))
        .unwrap_or(&base)
        .to_string();
    let base = base
        .strip_suffix(", the")
        .or_else(|| base.strip_suffix(", a"))
        .unwrap_or(&base)
        .to_string();
    base.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Region of a dump name, as one canonical word.
fn region(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    [
        ("europe", &["(europe", "(eu)", "(eu,", "(e)"][..]),
        ("usa", &["(usa", "(us)", "(us,", "(u)"][..]),
        ("japan", &["(japan", "(jp)", "(jp,", "(j)"][..]),
        ("world", &["(world"][..]),
        ("france", &["(france", "(fr)"][..]),
    ]
    .iter()
    .find(|(_, tags)| tags.iter().any(|t| lower.contains(t)))
    .map(|(r, _)| *r)
}

/// Best file for `rom_name` among same-title candidates.
pub fn best_match<'a>(candidates: &'a [String], rom_name: &str) -> Option<&'a str> {
    let wanted = region(rom_name);
    let score = |c: &str| {
        let lower = c.to_lowercase();
        let mut s = 0i32;
        if wanted.is_some() && region(c) == wanted {
            s += 4;
        }
        // No-Intro style ("(Europe)") over TOSEC style ("(1990)(Acclaim)(US)").
        if ["(europe", "(usa", "(japan", "(world", "(france"].iter().any(|t| lower.contains(t)) {
            s += 2;
        }
        if c.contains('[') {
            s -= 6; // bad dump, hack, translation…
        }
        if ["beta", "proto", "demo", "sample"].iter().any(|t| lower.contains(t)) {
            s -= 4;
        }
        s
    };
    candidates
        .iter()
        .max_by(|a, b| score(a).cmp(&score(b)).then(b.len().cmp(&a.len())))
        .map(String::as_str)
}

/// File names (without `.png`) of a libretro `Named_Boxarts/` directory page.
pub fn parse_listing(html: &str) -> Vec<String> {
    html.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .filter(|href| href.ends_with(".png") && !href.contains('/'))
        .filter_map(|href| urlencoding::decode(href).ok().map(|s| s[..s.len() - 4].to_string()))
        .collect()
}

fn build_index(names: Vec<String>) -> Index {
    let mut index: Index = HashMap::new();
    for name in names {
        let key = title_key(&name);
        if !key.is_empty() {
            index.entry(key).or_default().push(name);
        }
    }
    index
}

fn cache_file(system: &str) -> PathBuf {
    let safe: String = system.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    crate::emuworld_base_dir().join("cache").join("libretro").join(format!("{}.txt", safe))
}

fn read_fresh_cache(path: &PathBuf) -> Option<Vec<String>> {
    let age = std::fs::metadata(path).ok()?.modified().ok()?.elapsed().ok()?.as_secs();
    if age > INDEX_TTL_SECS {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.lines().map(str::to_string).filter(|l| !l.is_empty()).collect())
}

async fn index_for(client: &reqwest::Client, system: &str) -> Option<Arc<Index>> {
    static INDEXES: OnceLock<tokio::sync::Mutex<HashMap<String, Arc<Index>>>> = OnceLock::new();
    // Held while downloading: lookups of the same system wait for one download.
    let mut cache = INDEXES.get_or_init(Default::default).lock().await;
    if let Some(index) = cache.get(system) {
        return Some(index.clone());
    }
    let path = cache_file(system);
    let names = match read_fresh_cache(&path) {
        Some(names) => names,
        None => {
            let url = format!("https://thumbnails.libretro.com/{}/Named_Boxarts/", urlencoding::encode(system));
            push_log("INFO", &format!("Covers: liste libretro de '{}' téléchargée (correspondance approchée)", system));
            let resp = client.get(&url).timeout(std::time::Duration::from_secs(60)).send().await;
            let html = match resp {
                Ok(r) if r.status().is_success() => r.text().await.ok()?,
                failure => {
                    let why = match failure {
                        Ok(r) => format!("HTTP {}", r.status()),
                        Err(e) => e.to_string(),
                    };
                    push_log("WARN", &format!("Covers: liste libretro de '{}' indisponible ({}), ignorée jusqu'au prochain lancement", system, why));
                    // Remember the miss: libretro has no Switch list (404), it was
                    // downloaded again for every game.
                    cache.insert(system.to_string(), Arc::new(Index::new()));
                    return None;
                }
            };
            let names = parse_listing(&html);
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = crate::atomic_file::write(&path, names.join("\n").as_bytes());
            names
        }
    };
    let index = Arc::new(build_index(names));
    cache.insert(system.to_string(), index.clone());
    Some(index)
}

/// libretro file name (without `.png`) whose title matches `rom_name`.
pub async fn find(client: &reqwest::Client, system: &str, rom_name: &str) -> Option<String> {
    let key = title_key(rom_name);
    if key.len() < 3 {
        return None;
    }
    let index = index_for(client, system).await?;
    best_match(index.get(&key)?, rom_name).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_match_whatever_the_spelling() {
        assert_eq!(title_key("Duck Tales (Europe)"), title_key("DuckTales (Europe)"));
        assert_eq!(title_key("Skate or Die! (Europe)"), title_key("Skate Or Die (1988-12)(Ultra Soft)(US)"));
        assert_eq!(title_key("Chip 'n Dale Rescue Rangers (Europe)"), title_key("Chip 'n Dale - Rescue Rangers (Europe)"));
        assert_eq!(title_key("Wizards & Warriors III (Europe)"), title_key("Wizards _ Warriors III (USA)"));
        assert_eq!(title_key("Legend of Zelda, The (Europe)"), title_key("The Legend of Zelda (USA)"));
        // Sequels stay apart.
        assert_ne!(title_key("Duck Tales 2 (Europe)"), title_key("DuckTales (Europe)"));
        assert_ne!(title_key("Shadow Warriors Episode II (Europe)"), title_key("Shadow Warriors (Europe)"));
    }

    #[test]
    fn picks_the_closest_dump() {
        let candidates: Vec<String> = [
            "Skate Or Die (1988-12)(Ultra Soft)(US)[b2]",
            "Skate or Die (USA)",
            "Skate or Die (Europe) (Beta)",
            "Skate or Die (Europe)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(best_match(&candidates, "Skate or Die! (Europe)"), Some("Skate or Die (Europe)"));
        assert_eq!(best_match(&candidates, "Skate or Die! (USA)"), Some("Skate or Die (USA)"));
        // Only TOSEC-style names: still better than nothing, clean dump first.
        let tosec: Vec<String> = ["Pin-Bot (1990-04)(Nintendo)(US)[b]", "Pin-Bot (1990)(Nintendo)(EU)"].iter().map(|s| s.to_string()).collect();
        assert_eq!(best_match(&tosec, "Pin-Bot (Europe)"), Some("Pin-Bot (1990)(Nintendo)(EU)"));
    }

    /// Live: `cargo test libretro_live -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn libretro_live_nes_lookup() {
        let client = reqwest::Client::new();
        let roms = [
            "Duck Tales (Europe)", "Skate or Die! (Europe)", "Shadow Warriors (Europe)", "Pin-Bot (Europe)",
            "WWF Wrestlemania Steel Cage Challenge (Europe)", "Ironsword - Wizards & Warriors II (Europe)",
            "Rod Land (Europe)", "Tom & Jerry (and Tuffy) (Europe)", "Galaxy 5000 (Europe)",
        ];
        let mut found = 0;
        for rom in roms {
            let hit = find(&client, "Nintendo - Nintendo Entertainment System", rom).await;
            println!("{:45} -> {:?}", rom, hit);
            found += hit.is_some() as usize;
        }
        assert!(found >= 7, "{found}/9");
    }

    #[test]
    fn reads_the_libretro_directory_listing() {
        let html = r#"<a href="../">../</a>
<a href="DuckTales%20%28Europe%29.png">DuckTales (Europe).png</a>
<a href="Wizards%20_%20Warriors%20%28USA%29.png">x</a>
<a href="sub/dir.png">nested</a>"#;
        assert_eq!(parse_listing(html), ["DuckTales (Europe)", "Wizards _ Warriors (USA)"]);
        let index = build_index(parse_listing(html));
        assert_eq!(index[&title_key("Duck Tales (Europe)")], ["DuckTales (Europe)"]);
    }
}
