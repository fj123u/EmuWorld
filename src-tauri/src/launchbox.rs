//! LaunchBox Games Database (gamesdb.launchbox-app.com) as a late cover
//! source: very wide coverage (obscure European releases, Switch, Wii…).
//!
//! There is no public API: the search page lists games with their platform,
//! the game page lists its "Box - Front" images per region. The image CDN
//! serves ~200×286 thumbnails, fine for library cards; that's why this comes
//! after iiDB / tinfoil / nlib / libretro and only before Wikipedia.
//! Requests carry an honest EmuWorld user agent and go through the cover
//! lookup limit (6 at a time).

use crate::push_log;

const SITE: &str = "https://gamesdb.launchbox-app.com";
const IMAGE_HOST: &str = "https://images.launchbox-app.com/";

/// LaunchBox platform names for an EmuWorld console.
pub fn platforms(console: &str) -> &'static [&'static str] {
    match console {
        "NES" => &["Nintendo Entertainment System", "Nintendo Famicom Disk System"],
        "Super Nintendo" => &["Super Nintendo Entertainment System"],
        "Nintendo 64" => &["Nintendo 64"],
        "Game Boy Advance" => &["Nintendo Game Boy Advance", "Nintendo Game Boy Color", "Nintendo Game Boy"],
        "Nintendo DS" => &["Nintendo DS"],
        "Nintendo 3DS" => &["Nintendo 3DS"],
        "GameCube / Wii" => &["Nintendo GameCube", "Nintendo Wii"],
        "Wii U" => &["Nintendo Wii U"],
        "Nintendo Switch" => &["Nintendo Switch"],
        "Nintendo Switch 2" => &["Nintendo Switch 2"],
        "Virtual Boy" => &["Nintendo Virtual Boy"],
        "PlayStation 1" => &["Sony Playstation"],
        "PlayStation 2" => &["Sony Playstation 2"],
        "PlayStation 3" => &["Sony Playstation 3"],
        "PlayStation 4" => &["Sony Playstation 4"],
        "PlayStation 5" => &["Sony Playstation 5"],
        "PlayStation Portable" => &["Sony PSP"],
        "Xbox" => &["Microsoft Xbox"],
        "Xbox 360" => &["Microsoft Xbox 360"],
        "Master System" => &["Sega Master System"],
        "Mega Drive" => &["Sega Genesis"],
        "Sega CD" => &["Sega CD"],
        "Sega 32X" => &["Sega 32X"],
        "Saturn" => &["Sega Saturn"],
        "Dreamcast" => &["Sega Dreamcast"],
        "Game Gear" => &["Sega Game Gear"],
        "Atari 2600" => &["Atari 2600"],
        "Atari 5200" => &["Atari 5200"],
        "Atari 7800" => &["Atari 7800"],
        "Jaguar" => &["Atari Jaguar", "Atari Jaguar CD"],
        "Lynx" => &["Atari Lynx"],
        "Neo-Geo" => &["SNK Neo Geo AES", "SNK Neo Geo MVS", "SNK Neo Geo CD"],
        "Arcade" => &["Arcade"],
        "PC Engine" | "TurboGrafx-16" => &["NEC TurboGrafx-16"],
        "TurboGrafx-CD" => &["NEC TurboGrafx-CD"],
        "CD-i" => &["Philips CD-i"],
        "WonderSwan" => &["Bandai WonderSwan", "Bandai WonderSwan Color"],
        _ => &[],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub title: String,
    pub platform: String,
    /// "/games/details/1241-ducktales"
    pub details_path: String,
}

fn unescape(s: &str) -> String {
    s.replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .trim()
        .to_string()
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = s.find(start)? + start.len();
    let to = s[from..].find(end)? + from;
    Some(&s[from..to])
}

/// Results of a search page: one per game card (title, platform, link).
pub fn parse_search(html: &str) -> Vec<SearchHit> {
    html.split("class=\"games-grid-card")
        .skip(1)
        .filter_map(|card| {
            let details_path = between(card, "href=\"", "\"")?.to_string();
            if !details_path.starts_with("/games/details/") || details_path.contains("..") {
                return None;
            }
            let title_block = card.split("<h3").nth(1)?;
            let title = unescape(between(title_block, ">", "</h3>")?);
            let platform = unescape(between(title_block.split("</h3>").nth(1)?, ">", "</p>")?);
            Some(SearchHit { title, platform, details_path })
        })
        .collect()
}

/// "Box - Front" images of a game page: (region, image URL).
pub fn parse_box_fronts(html: &str) -> Vec<(String, String)> {
    html.split("<img src=\"")
        .skip(1)
        .filter_map(|tag| {
            let url = tag.split('"').next()?;
            let alt = between(tag, "alt=\"", "\"")?;
            if !url.starts_with(IMAGE_HOST) || alt.contains("Fanart") || !alt.contains(" - Box - Front (") {
                return None;
            }
            let region = between(alt, " - Box - Front (", ")")?.to_string();
            let file = url.trim_start_matches(IMAGE_HOST).trim_start_matches('/');
            let safe = !file.is_empty() && file.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
            safe.then(|| (region, format!("{}{}", IMAGE_HOST, file)))
        })
        .collect()
}

/// The full-size picture behind a thumbnail of a game page.
///
/// The page shows ~170 px thumbnails (blurry once stretched on a card). Its
/// embedded data lists, right after each thumbnail's file name, the file of
/// the original image: `"<thumb>.jpg","Box - Front",11,171,281,"Europe","<full>.jpg",…`.
pub fn full_size_image(page: &str, thumbnail_url: &str) -> Option<String> {
    let file = thumbnail_url.rsplit('/').next()?;
    let quoted = format!("\"{}\"", file);
    let after = &page[page.find(&quoted)? + quoted.len()..];
    let window = &after[..after.len().min(600)];
    window.split('"').find_map(|token| {
        let (stem, ext) = token.rsplit_once('.')?;
        let is_image_file = matches!(ext, "jpg" | "jpeg" | "png" | "webp")
            && stem.len() == 36
            && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        (is_image_file && token != file).then(|| format!("{}{}", IMAGE_HOST, token))
    })
}

/// LaunchBox regions to prefer for a ROM, from its tags.
fn preferred_regions(rom_name: &str) -> &'static [&'static str] {
    let lower = rom_name.to_lowercase();
    if ["(europe", "(france", "(germany", "(spain", "(italy", "(uk", "(eu)", "(fr)"].iter().any(|t| lower.contains(t)) {
        &["Europe", "France", "United Kingdom", "Germany", "Spain", "Italy", "World"]
    } else if ["(japan", "(jp)", "(j)"].iter().any(|t| lower.contains(t)) {
        &["Japan", "World"]
    } else {
        &["North America", "United States", "World"]
    }
}

pub fn pick_box_front(fronts: &[(String, String)], rom_name: &str) -> Option<String> {
    preferred_regions(rom_name)
        .iter()
        .find_map(|r| fronts.iter().find(|(region, _)| region == r))
        .or_else(|| fronts.first())
        .map(|(_, url)| url.clone())
}

/// Search hit for the ROM: right platform, and the same title (ignoring
/// case/punctuation), or the same main title before ":" ("Galaxy 5000:
/// Racing in the 51st Century"), or — when it is the only candidate — a title
/// that just adds a name around it without any number ("Capcom's Gold Medal
/// Challenge '92", or the European "Mario Strikers Charged Football").
/// Sequels never match.
pub fn pick_hit<'a>(hits: &'a [SearchHit], rom_title: &str, console: &str) -> Option<&'a SearchHit> {
    let wanted = platforms(console);
    let key = crate::libretro_index::title_key(rom_title);
    if key.len() < 3 {
        return None;
    }
    let on_platform: Vec<&SearchHit> = hits.iter().filter(|h| wanted.contains(&h.platform.as_str())).collect();
    let main_title = |t: &str| crate::libretro_index::title_key(t.split(':').next().unwrap_or(t));
    if let Some(h) = on_platform.iter().find(|h| crate::libretro_index::title_key(&h.title) == key) {
        return Some(h);
    }
    if let Some(h) = on_platform.iter().find(|h| main_title(&h.title) == key) {
        return Some(h);
    }
    let loose: Vec<&&SearchHit> = on_platform
        .iter()
        .filter(|h| {
            let k = crate::libretro_index::title_key(&h.title);
            let no_digit = |extra: String| !extra.chars().any(|c| c.is_ascii_digit());
            // LaunchBox adds words ("Capcom's …"), or the ROM does
            // ("Mario Strikers Charged Football", the European title).
            (key.len() >= 6 && k.contains(&key) && no_digit(k.replacen(&key, "", 1)))
                || (k.len() >= 10 && key.starts_with(&k) && no_digit(key[k.len()..].to_string()))
        })
        .collect();
    if loose.len() == 1 { Some(loose[0]) } else { None }
}

fn user_agent() -> String {
    format!("EmuWorld/{} (+https://github.com/fj123u/EmuWorld)", env!("CARGO_PKG_VERSION"))
}

/// URL of a LaunchBox box art for the ROM, if a confident match exists.
pub async fn find_box_url(client: &reqwest::Client, rom_name: &str, console: &str) -> Result<Option<String>, String> {
    if platforms(console).is_empty() {
        return Ok(None);
    }
    let title = crate::iidb::title_from_rom_name(rom_name);
    if title.len() < 3 {
        return Ok(None);
    }
    let search_url = format!("{}/games/results/{}", SITE, urlencoding::encode(&title.to_lowercase()));
    let html = client
        .get(&search_url)
        .header(reqwest::header::USER_AGENT, user_agent())
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("LaunchBox search: {}", e))?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let hits = parse_search(&html);
    let Some(hit) = pick_hit(&hits, &title, console) else {
        return Ok(None);
    };
    let page = client
        .get(format!("{}{}", SITE, hit.details_path))
        .header(reqwest::header::USER_AGENT, user_agent())
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("LaunchBox game page: {}", e))?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let url = pick_box_front(&parse_box_fronts(&page), rom_name)
        .map(|thumbnail| full_size_image(&page, &thumbnail).unwrap_or(thumbnail));
    if url.is_some() {
        push_log("INFO", &format!("LaunchBox: '{}' ({}) → '{}'", rom_name, console, hit.title));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = r#"
<div b-x class="games-grid-card col-xl-3"> <a class="list-item" href="/games/details/1241-ducktales"> <div class="gameCard"> <div class="cardTitle"> <h3 b-x>DuckTales</h3> <p b-x>Nintendo Entertainment System</p> </div></div></a></div>
<div b-x class="games-grid-card col-xl-3"> <a class="list-item" href="/games/details/9999-ducktales-2"> <div class="cardTitle"> <h3 b-x>DuckTales 2</h3> <p b-x>Nintendo Entertainment System</p> </div></a></div>
<div b-x class="games-grid-card col-xl-3"> <a class="list-item" href="/games/details/5555-ducktales"> <div class="cardTitle"> <h3 b-x>DuckTales</h3> <p b-x>Nintendo Game Boy</p> </div></a></div>
<div b-x class="games-grid-card col-xl-3"> <a class="list-item" href="/games/details/7777-gold"> <div class="cardTitle"> <h3 b-x>Capcom&#x27;s Gold Medal Challenge &#x27;92</h3> <p b-x>Nintendo Entertainment System</p> </div></a></div>
<div b-x class="games-grid-card col-xl-3"> <a class="list-item" href="/games/details/8888-galaxy"> <div class="cardTitle"> <h3 b-x>Galaxy 5000: Racing in the 51st Century</h3> <p b-x>Nintendo Entertainment System</p> </div></a></div>
"#;

    #[test]
    fn reads_search_results() {
        let hits = parse_search(SEARCH);
        assert_eq!(hits.len(), 5);
        assert_eq!(hits[0], SearchHit { title: "DuckTales".into(), platform: "Nintendo Entertainment System".into(), details_path: "/games/details/1241-ducktales".into() });
        assert_eq!(hits[3].title, "Capcom's Gold Medal Challenge '92");
    }

    #[test]
    fn matches_the_right_game_on_the_right_platform() {
        let hits = parse_search(SEARCH);
        let pick = |t: &str, c: &str| pick_hit(&hits, t, c).map(|h| h.details_path.as_str());
        assert_eq!(pick("Duck Tales", "NES"), Some("/games/details/1241-ducktales"));
        assert_eq!(pick("Duck Tales", "Game Boy Advance"), Some("/games/details/5555-ducktales"));
        assert_eq!(pick("Galaxy 5000", "NES"), Some("/games/details/8888-galaxy"));
        assert_eq!(pick("Gold Medal Challenge '92", "NES"), Some("/games/details/7777-gold"));
        assert_eq!(pick("Duck Tales", "Nintendo 64"), None, "wrong platform");
        assert_eq!(pick("Duck Tales 3", "NES"), None, "never another sequel");
        let strikers = vec![SearchHit { title: "Mario Strikers Charged".into(), platform: "Nintendo Wii".into(), details_path: "/games/details/1-msc".into() }];
        assert!(pick_hit(&strikers, "Mario Strikers Charged Football", "GameCube / Wii").is_some());
        let mario = vec![SearchHit { title: "Super Mario Bros.".into(), platform: "Nintendo Entertainment System".into(), details_path: "/games/details/2-smb".into() }];
        assert!(pick_hit(&mario, "Super Mario Bros. 3", "NES").is_none(), "a longer ROM title with a number is a sequel");
    }

    #[test]
    fn picks_the_box_of_the_rom_region() {
        let page = r#"<img src="https://images.launchbox-app.com//776785ad-c738.jpg" class="m" alt="DuckTales - Box - Front (North America) - 1464x2100" loading="lazy">
<img src="https://images.launchbox-app.com//f11f58f7-f0d8.png" alt="DuckTales - Box - Front (Germany) - 800x1146">
<img src="https://images.launchbox-app.com//c59581ba.png" alt="DuckTales - Fanart - Box - Front (null) - 500x646">
<img src="https://evil.example/x.png" alt="DuckTales - Box - Front (Europe) - 1x1">"#;
        let fronts = parse_box_fronts(page);
        assert_eq!(fronts.len(), 2, "fanart and foreign hosts are ignored");
        assert_eq!(pick_box_front(&fronts, "Duck Tales (Europe)").as_deref(), Some("https://images.launchbox-app.com/f11f58f7-f0d8.png"));
        assert_eq!(pick_box_front(&fronts, "Duck Tales (USA)").as_deref(), Some("https://images.launchbox-app.com/776785ad-c738.jpg"));
        assert_eq!(pick_box_front(&fronts, "Duck Tales (Japan)").as_deref(), Some("https://images.launchbox-app.com/776785ad-c738.jpg"), "falls back to any box");
    }

    #[test]
    fn takes_the_full_size_image_behind_a_thumbnail() {
        // Shape of a real game page (Nintendo Switch Sports Resort, 11/2026).
        let page = r#"<img src="https://images.launchbox-app.com//ae45ebae-291d-4827-86ac-359079f2fd9a.jpg" alt="Nintendo Switch Sports Resort - Box - Front (Europe) - 1162x1904">
            …{"imageFileName":75,"fullGameImageFileName":81},"ae45ebae-291d-4827-86ac-359079f2fd9a.jpg","Box - Front",11,171,281,"Europe","25107e31-c930-45eb-a52b-f31caedc5e35.jpg",20332969,1162,1904,{"x":1}"#;
        assert_eq!(
            full_size_image(page, "https://images.launchbox-app.com/ae45ebae-291d-4827-86ac-359079f2fd9a.jpg").as_deref(),
            Some("https://images.launchbox-app.com/25107e31-c930-45eb-a52b-f31caedc5e35.jpg"));
        // Unknown thumbnail, or a page without the embedded data: keep the thumbnail.
        assert_eq!(full_size_image(page, "https://images.launchbox-app.com/00000000-0000-0000-0000-000000000000.jpg"), None);
        assert_eq!(full_size_image("<img src=\"x\">", "https://images.launchbox-app.com/ae45ebae-291d-4827-86ac-359079f2fd9a.jpg"), None);
        // Nothing but an image file name of that host is ever taken from the page.
        let odd = r#""ae45ebae-291d-4827-86ac-359079f2fd9a.jpg","https://evil.example/x.jpg","../../etc/passwd.png",1"#;
        assert_eq!(full_size_image(odd, "https://images.launchbox-app.com/ae45ebae-291d-4827-86ac-359079f2fd9a.jpg"), None);
    }

    #[test]
    fn every_catalog_console_has_a_launchbox_platform() {
        let missing: Vec<String> = crate::emulators::get_catalog()
            .into_iter()
            .map(|e| e.console)
            .filter(|c| c != "Multi-System" && platforms(c).is_empty())
            .collect();
        assert!(missing.is_empty(), "add them to launchbox::platforms: {missing:?}");
    }

    /// Live: `cargo test launchbox_live -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn launchbox_live_lookup() {
        let client = reqwest::Client::new();
        for (rom, console) in [
            ("Galaxy 5000 (Europe)", "NES"),
            ("Gold Medal Challenge '92 (Europe)", "NES"),
            ("Mario Strikers Charged Football (Europe) (En,Fr,De,Es,It) (v1.01)", "GameCube / Wii"),
            ("Luigi's Mansion 2 HD [010048701995E000][v0][US](nsw2u.com)", "Nintendo Switch"),
        ] {
            println!("{:60} -> {:?}", rom, find_box_url(&client, rom, console).await);
        }
    }
}
