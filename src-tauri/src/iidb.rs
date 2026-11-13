//! iiDB (https://iidb.iisu.network) — the iiSU community game-art database.
//!
//! Title helpers. iiDB itself is queried in iidb_api.rs, through the
//! official API only: its terms (section 8) forbid the website's internal
//! endpoint used up to 2.6.4; a test in lib.rs fails if it comes back.
//!
//! Matching is deliberately strict: a game is accepted only when its title
//! normalizes to exactly the ROM's title *and* it exists on the ROM's
//! platform. A missing cover falls back to another source; a wrong cover
//! would just be wrong, so ambiguity always means "no match".
#![allow(dead_code)]

use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

/// Host serving iiDB images (never hotlinked: not allow-listed for Discord).
pub const ASSETS_HOST: &str = "assets.iisu.network";

/// One search result (only the fields EmuWorld uses).
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Game {
    pub name: String,
    #[serde(default)]
    pub primary_platform: Option<String>,
    /// Comma-separated platform names, e.g. "GameCube, Wii".
    #[serde(default)]
    pub platform_list: Option<String>,
    #[serde(default)]
    pub is_rom_hack: bool,
    /// amiibo and other non-game "extras" that show up in searches.
    #[serde(default)]
    pub is_search_extra: bool,
    /// Path of the main box art under `https://assets.iisu.network/`.
    #[serde(default)]
    pub boxart_filename: Option<String>,
}

/// iiDB platform names for an EmuWorld console (as listed in the API's
/// `platform_list`). Several spellings where iiDB uses more than one.
/// Empty = console not mapped: iiDB is skipped for it.
pub fn platforms(console: &str) -> &'static [&'static str] {
    match console {
        "NES" | "Famicom" => &["Nintendo Entertainment System", "Family Computer", "Family Computer Disk System"],
        "SNES" | "Super Nintendo" | "Super Famicom" => &["Super Nintendo Entertainment System", "Super Famicom"],
        "Nintendo 64" | "N64" => &["Nintendo 64"],
        // EmuWorld files Game Boy / Color games under the GBA emulators.
        "Game Boy Advance" | "GBA" | "Game Boy" | "Game Boy Color" => &["Game Boy Advance", "Game Boy", "Game Boy Color"],
        "Nintendo DS" => &["Nintendo DS", "Nintendo DSi"],
        "Nintendo 3DS" | "3DS" => &["Nintendo 3DS", "New Nintendo 3DS"],
        "GameCube / Wii" | "GameCube - Wii" | "GameCube" | "Wii" => &["GameCube", "Wii"],
        "Wii U" => &["Wii U"],
        "Nintendo Switch" => &["Nintendo Switch"],
        "Nintendo Switch 2" => &["Switch 2", "Nintendo Switch 2"],
        "Virtual Boy" => &["Virtual Boy"],
        "PlayStation 1" | "PS1" => &["PlayStation"],
        "PlayStation 2" | "PS2" => &["PlayStation 2"],
        "PlayStation 3" | "PS3" => &["PlayStation 3"],
        "PlayStation 4" | "PS4" => &["PlayStation 4"],
        "PlayStation 5" | "PS5" => &["PlayStation 5"],
        "PlayStation Portable" | "PSP" => &["PlayStation Portable"],
        "Xbox" => &["Xbox"],
        "Xbox 360" => &["Xbox 360"],
        "Mega Drive" | "Genesis" => &["Sega Genesis", "Sega Mega Drive"],
        "Master System" => &["Sega Master System"],
        "Game Gear" => &["Game Gear", "Sega Game Gear"],
        "Sega CD" => &["Sega CD", "Sega Mega-CD"],
        "Sega 32X" => &["Sega Mega Drive 32X", "Sega 32X"],
        "Saturn" => &["Sega Saturn"],
        "Dreamcast" => &["Dreamcast", "Sega Dreamcast"],
        "TurboGrafx-16" | "PC Engine" => &["NEC PC Engine", "TurboGrafx-16", "NEC TurboGrafx-16"],
        "TurboGrafx-CD" => &["NEC PC Engine CD", "TurboGrafx-CD", "NEC TurboGrafx-CD", "NEC PC Engine"],
        "Neo-Geo" => &["Neo Geo", "Neo Geo AES", "Neo Geo MVS", "Neo Geo CD"],
        "Arcade" => &["Arcade"],
        "Atari 2600" => &["Atari 2600"],
        "Atari 5200" => &["Atari 5200"],
        "Atari 7800" => &["Atari 7800"],
        "Jaguar" => &["Atari Jaguar", "Atari Jaguar CD"],
        "Lynx" => &["Atari Lynx"],
        "WonderSwan" => &["WonderSwan", "WonderSwan Color", "Bandai WonderSwan", "Bandai WonderSwan Color"],
        "CD-i" => &["CD-i", "Philips CD-i"],
        _ => &[],
    }
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

/// Drops what some dump sites add around a title: a trailing "ROM"
/// ("Donkey Kong Bananza ROM"). Only as the last word, in capitals: a game
/// called "ROM Check Fail" keeps its name.
pub fn without_dump_words(title: &str) -> String {
    let trimmed = title.trim_end();
    match trimmed.strip_suffix("ROM") {
        Some(rest) if rest.ends_with(char::is_whitespace) && !rest.trim().is_empty() => rest.trim_end().to_string(),
        _ => title.to_string(),
    }
}

/// Human title of a ROM file name: drops region/language/version tags,
/// Switch title ids, leftover inner extensions, and fixes "X, The" titles.
/// Unlike `clean_game_name` (used for fuzzy web searches) it never cuts a
/// subtitle, so "Zelda - Ocarina of Time" stays distinct from "Zelda".
pub fn title_from_rom_name(rom_name: &str) -> String {
    static TAGS: OnceLock<Regex> = OnceLock::new();
    static EXT: OnceLock<Regex> = OnceLock::new();
    static THE: OnceLock<Regex> = OnceLock::new();
    let mut t = rom_name.replace('_', " ");
    t = re(&EXT, r"(?i)\.(xiso|nkit|iso|dec|trim|nsp|nup|xci|nsz|wud|wux|rvz|wbfs|cia|3ds)$").replace(&t, "").to_string();
    t = re(&TAGS, r"\([^)]*\)|\[[^\]]*\]").replace_all(&t, " ").to_string();
    let t = without_dump_words(&t.split_whitespace().collect::<Vec<_>>().join(" "));
    // No-Intro / Redump put the article last: "Legend of Zelda, The - Ocarina of Time".
    re(&THE, r"(?i)^(.+?), (the|a|an)\b(.*)$")
        .replace(&t, "$2 $1$3")
        .trim()
        .to_string()
}

fn fold_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'ō' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            _ => c,
        })
        .collect()
}

/// Comparison key for titles: case, accents, punctuation, "&"/"and", a
/// leading "the" and the word "version" are ignored.
/// "Pokémon: Emerald Version" == "Pokemon - Emerald" == "pokemon emerald".
pub fn normalize_title(title: &str) -> String {
    let lower = fold_accents(&title.to_lowercase()).replace('&', " and ");
    let mut words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && *w != "version")
        .collect();
    if words.first() == Some(&"the") {
        words.remove(0);
    }
    words.concat()
}

/// French Pokémon titles (common in FR libraries, e.g. "pokemon lune") →
/// the English names iiDB uses. Applied to Pokémon titles only: these words
/// are too generic to translate anywhere else.
fn translate_french_pokemon(title: &str) -> Option<String> {
    let lower = fold_accents(&title.to_lowercase());
    if !lower.contains("pokemon") {
        return None;
    }
    // Multi-word entries first: they change word order.
    const PAIRS: &[(&str, &str)] = &[
        ("rubis omega", "omega ruby"), ("saphir alpha", "alpha sapphire"),
        ("or heartgold", "heartgold"), ("argent soulsilver", "soulsilver"),
        ("rouge feu", "firered"), ("vert feuille", "leafgreen"),
        ("lune", "moon"), ("soleil", "sun"), ("rouge", "red"), ("bleu", "blue"),
        ("jaune", "yellow"), ("argent", "silver"), ("or", "gold"), ("cristal", "crystal"),
        ("rubis", "ruby"), ("saphir", "sapphire"), ("emeraude", "emerald"),
        ("diamant", "diamond"), ("perle", "pearl"), ("platine", "platinum"),
        ("noir", "black"), ("blanc", "white"), ("epee", "sword"), ("bouclier", "shield"),
        ("ecarlate", "scarlet"),
    ];
    let mut out = format!(" {} ", lower.split_whitespace().collect::<Vec<_>>().join(" "));
    for (fr, en) in PAIRS {
        out = out.replace(&format!(" {} ", fr), &format!(" {} ", en));
    }
    let out = out.trim().to_string();
    (out != lower.trim()).then_some(out)
}

/// Scene-style release names ("1.2.Switch.PROPER.NSW-BigBlueBox") use dots
/// instead of spaces and end with group tags: rebuild a plain title.
fn title_from_scene_name(title: &str) -> Option<String> {
    // The release itself is the first word; anything after a space is the
    // group/uploader ("… kaze-nico").
    let title = title.split_whitespace().next()?;
    if title.matches('.').count() < 2 {
        return None;
    }
    const SCENE_TAGS: &[&str] = &[
        "proper", "repack", "nsw", "multi", "readnfo", "internal", "update", "dlc",
        "xci", "nsp", "nsz", "eur", "usa", "jpn",
    ];
    let words: Vec<&str> = title
        .split(['.', '-'])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .take_while(|w| !SCENE_TAGS.contains(&w.to_lowercase().as_str()))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// Titles to search for, most specific first (at most two requests per game).
pub fn query_variants(rom_name: &str) -> Vec<String> {
    let title = title_from_rom_name(rom_name);
    let title = title_from_scene_name(&title).unwrap_or(title);
    let mut out = vec![];
    if !title.is_empty() {
        out.push(title.clone());
    }
    if let Some(en) = translate_french_pokemon(&title) {
        if normalize_title(&en) != normalize_title(&title) {
            out.push(en);
        }
    }
    out
}

fn on_platform(game: &Game, wanted: &[&str]) -> bool {
    let list = game.platform_list.as_deref().unwrap_or("");
    list.split(',')
        .map(str::trim)
        .chain(game.primary_platform.as_deref())
        .any(|p| wanted.iter().any(|w| w.eq_ignore_ascii_case(p)))
}

/// iiDB disambiguates some titles with a suffix — "Tomb Raider (2013)",
/// "Mario vs. Donkey Kong (2024)" — which a ROM name never has.
fn iidb_title(name: &str) -> String {
    title_from_rom_name(name)
}

/// Words of a title, normalized like `normalize_title` but kept separate.
fn title_words(title: &str) -> Vec<String> {
    let lower = fold_accents(&title.to_lowercase()).replace('&', " and ").replace('\'', "");
    let mut words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && *w != "version")
        .map(String::from)
        .collect();
    if words.first().map(String::as_str) == Some("the") {
        words.remove(0);
    }
    words
}

/// Loose match for informal file names ("zelda majoras mask" → "The Legend
/// of Zelda - Majora's Mask 3D"): every word of the ROM title appears in
/// the iiDB title, which adds at most a few words. Only used when it
/// singles out one game, see `pick`.
fn contains_all_words(rom_title: &str, iidb_name: &str) -> bool {
    const MAX_EXTRA_WORDS: usize = 3;
    // An extra sequel number means another game: "Metroid Prime" is not
    // "Metroid Prime 2: Echoes". ("3D" / "HD" remakes are fine.)
    fn is_sequel_number(w: &str) -> bool {
        w.chars().all(|c| c.is_ascii_digit())
            || matches!(w, "ii" | "iii" | "iv" | "vi" | "vii" | "viii" | "ix" | "xi" | "xii")
    }
    let wanted = title_words(rom_title);
    let have = title_words(&iidb_title(iidb_name));
    wanted.len() >= 2
        && wanted.iter().all(|w| have.contains(w))
        && have.len() <= wanted.len() + MAX_EXTRA_WORDS
        && !have.iter().any(|w| !wanted.contains(w) && is_sequel_number(w))
}

/// Best result for the ROM, or `None` when nothing matches confidently.
/// Candidates need a box art and the ROM's platform. Then:
/// 1. a title that normalizes exactly like the ROM's wins (real games
///    before ROM hacks of the same name);
/// 2. otherwise a loose all-words match is accepted only if exactly one
///    candidate qualifies — two ("Tomb Raider II", "Tomb Raider III") means
///    we can't tell, so the caller falls back to other sources.
pub fn pick<'a>(items: &'a [Game], titles: &[String], wanted_platforms: &[&str]) -> Option<&'a Game> {
    let keys: Vec<String> = titles.iter().map(|t| normalize_title(t)).filter(|k| !k.is_empty()).collect();
    let usable: Vec<&Game> = items
        .iter()
        .filter(|g| !g.is_search_extra)
        .filter(|g| g.boxart_filename.as_deref().is_some_and(|f| asset_url(f).is_some()))
        .filter(|g| on_platform(g, wanted_platforms))
        .collect();

    let mut exact: Vec<&Game> = usable
        .iter()
        .copied()
        .filter(|g| keys.contains(&normalize_title(&g.name)) || keys.contains(&normalize_title(&iidb_title(&g.name))))
        .collect();
    exact.sort_by_key(|g| g.is_rom_hack);
    if let Some(best) = exact.first() {
        return Some(best);
    }

    let loose: Vec<&Game> = usable
        .iter()
        .copied()
        .filter(|g| !g.is_rom_hack)
        .filter(|g| titles.iter().any(|t| contains_all_words(t, &g.name)))
        .collect();
    (loose.len() == 1).then(|| loose[0])
}

/// Full image URL for a `boxart_filename`, refusing anything that is not a
/// plain relative path on the iiDB asset host.
pub fn asset_url(filename: &str) -> Option<String> {
    let ok = !filename.is_empty()
        && !filename.contains("..")
        && !filename.starts_with('/')
        && filename.chars().all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c));
    ok.then(|| format!("https://{}/{}", ASSETS_HOST, filename))
}

#[cfg(test)]
mod tests {
    use super::*;


    fn game(name: &str, platforms: &str, boxart: Option<&str>) -> Game {
        Game {
            name: name.into(),
            primary_platform: platforms.split(',').next().map(|s| s.trim().to_string()),
            platform_list: Some(platforms.into()),
            boxart_filename: boxart.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn rom_names_become_titles() {
        assert_eq!(title_from_rom_name("Super Mario Bros. 3 (Europe)"), "Super Mario Bros. 3");
        assert_eq!(title_from_rom_name("Halo 2 (France) (En,Ja,Fr,De,Es,It,Zh,Ko).xiso"), "Halo 2");
        assert_eq!(
            title_from_rom_name("Legend of Zelda, The - Ocarina of Time (USA) (Rev 2)"),
            "The Legend of Zelda - Ocarina of Time"
        );
        assert_eq!(title_from_rom_name("Pokemon Sword [0100ABF008968000][v0] (Incl. 2 DLCs)"), "Pokemon Sword");
        assert_eq!(title_from_rom_name("pac man et l'aventure des fantômes "), "pac man et l'aventure des fantômes");
        assert_eq!(title_from_rom_name("Mario_Kart_8"), "Mario Kart 8");
        // Switch 2 dumps: "ROM" appended, a leading "[Game]" tag, a version in parentheses.
        assert_eq!(title_from_rom_name("Donkey Kong Bananza ROM [040074A01BF12000]"), "Donkey Kong Bananza");
        assert_eq!(title_from_rom_name("Mario Tennis Fever  ROM [0400D4201C6EA000]"), "Mario Tennis Fever");
        assert_eq!(title_from_rom_name("[Game] Pokemon Pokopia (v2.0.1)[0400A3A01EE4C000]"), "Pokemon Pokopia");
        assert_eq!(title_from_rom_name("ROM Check Fail"), "ROM Check Fail");
    }

    #[test]
    fn titles_normalize_across_spellings() {
        let a = normalize_title("Pokémon: Emerald Version");
        assert_eq!(a, normalize_title("Pokemon - Emerald"));
        assert_eq!(a, normalize_title("pokemon emerald"));
        assert_eq!(
            normalize_title("The Legend of Zelda: Ocarina of Time"),
            normalize_title("Legend of Zelda - Ocarina of Time")
        );
        assert_eq!(normalize_title("Banjo & Kazooie"), normalize_title("Banjo and Kazooie"));
        // A subtitle is part of the identity.
        assert_ne!(normalize_title("The Legend of Zelda"), normalize_title("The Legend of Zelda - Ocarina of Time"));
        assert_ne!(normalize_title("Metroid Prime"), normalize_title("Metroid Prime 2: Echoes"));
    }

    #[test]
    fn french_pokemon_titles_get_an_english_variant() {
        assert_eq!(query_variants("pokemon lune"), vec!["pokemon lune".to_string(), "pokemon moon".to_string()]);
        assert_eq!(query_variants("pokemon rubis omega")[1], "pokemon omega ruby");
        assert_eq!(query_variants("Pokémon - Version Rouge (France)")[1], "pokemon - version red");
        assert_eq!(query_variants("Pokemon Ultra Lune")[1], "pokemon ultra moon");
        // Non-Pokémon titles are never translated ("Or" / "Rouge" are generic words).
        assert_eq!(query_variants("Rouge Or Noir"), vec!["Rouge Or Noir".to_string()]);
    }

    #[test]
    fn pick_requires_exact_title_platform_and_art() {
        let items = vec![
            game("Pokemon - Emerald Advanced", "Game Boy Advance", Some("games/1/boxart/a.png")),
            game("Pokémon Emerald", "Game Boy Advance", Some("games/2/boxart/b.png")),
        ];
        let got = pick(&items, &["Pokemon - Emerald Version".into()], platforms("Game Boy Advance"));
        assert_eq!(got.map(|g| g.name.as_str()), Some("Pokémon Emerald"));

        // Right title, wrong platform → no match (falls back to other sources).
        let n64_only = vec![game("Super Mario 64", "Nintendo 64, Wii, Wii U", Some("games/3/boxart/c.png"))];
        assert!(pick(&n64_only, &["Super Mario 64".into()], platforms("Nintendo DS")).is_none());

        // A similar title is not the same game.
        let prime2 = vec![game("Metroid Prime 2: Echoes", "GameCube, Wii", Some("games/4/boxart/d.png"))];
        assert!(pick(&prime2, &["Metroid Prime".into()], platforms("GameCube / Wii")).is_none());

        // No box art → unusable.
        let no_art = vec![game("Metroid Prime", "GameCube", None)];
        assert!(pick(&no_art, &["Metroid Prime".into()], platforms("GameCube / Wii")).is_none());
    }

    #[test]
    fn pick_prefers_real_game_over_hack_and_skips_extras() {
        let mut hack = game("Super Mario Bros. 3", "Nintendo Entertainment System", Some("games/9/boxart/h.png"));
        hack.is_rom_hack = true;
        let mut amiibo = game("Super Mario Bros. 3", "Nintendo Entertainment System", Some("games/8/boxart/x.png"));
        amiibo.is_search_extra = true;
        let real = game("Super Mario Bros. 3", "Arcade, Nintendo Entertainment System, Wii", Some("games/95324/boxart/IFStolxe.png"));
        let items = vec![hack, amiibo, real];
        let got = pick(&items, &["Super Mario Bros. 3".into()], platforms("NES")).unwrap();
        assert_eq!(got.boxart_filename.as_deref(), Some("games/95324/boxart/IFStolxe.png"));
    }

    #[test]
    fn iidb_disambiguation_suffix_is_ignored() {
        // PS3 "Tomb Raider" ROM vs iiDB's "Tomb Raider (2013)"; the 1996 game
        // is not on PS3, so the platform filter keeps it out.
        let items = vec![
            game("Tomb Raider", "Sega Saturn, PlayStation, PC", Some("games/1/boxart/a.png")),
            game("Tomb Raider (2013)", "Xbox 360, PlayStation 3, PC", Some("games/2/boxart/b.png")),
        ];
        let got = pick(&items, &query_variants("Tomb Raider (Europe) (En,Ar)"), platforms("PlayStation 3"));
        assert_eq!(got.map(|g| g.name.as_str()), Some("Tomb Raider (2013)"));
    }

    #[test]
    fn informal_names_match_only_when_unambiguous() {
        let mm = vec![
            game("The Legend of Zelda - Majora's Mask", "Nintendo 64", Some("games/1/boxart/a.png")),
            game("The Legend of Zelda - Majora's Mask 3D", "Nintendo 3DS", Some("games/2/boxart/b.png")),
        ];
        let got = pick(&mm, &query_variants("zelda majoras mask"), platforms("Nintendo 3DS"));
        assert_eq!(got.map(|g| g.name.as_str()), Some("The Legend of Zelda - Majora's Mask 3D"));

        // Two sequels contain every word: ambiguous → no guess.
        let tr = vec![
            game("Tomb Raider II - Starring Lara Croft", "PlayStation", Some("games/3/boxart/c.png")),
            game("Tomb Raider III - Adventures of Lara Croft", "PlayStation", Some("games/4/boxart/d.png")),
        ];
        assert!(pick(&tr, &query_variants("Tomb Raider Lara Croft"), platforms("PlayStation 1")).is_none());

        // A single word is too vague to match loosely.
        let z = vec![game("Zelda's Adventure", "CD-i", Some("games/5/boxart/e.png"))];
        assert!(pick(&z, &query_variants("Zelda"), platforms("CD-i")).is_none());

        // Too many extra words: a different, longer game.
        let long = vec![game("Mario Kart 8 Deluxe Booster Course Pass Wave Edition", "Nintendo Switch", Some("games/6/boxart/f.png"))];
        assert!(pick(&long, &query_variants("Mario Kart"), platforms("Nintendo Switch")).is_none());
    }

    #[test]
    fn scene_release_names_are_rebuilt() {
        assert_eq!(query_variants("1.2.Switch.PROPER.NSW-BigBlueBox kaze-nico")[0], "1 2 Switch");
        assert_eq!(normalize_title("1 2 Switch"), normalize_title("1-2-Switch"));
        // Normal titles with a dot are untouched.
        assert_eq!(query_variants("Dr. Mario (Europe)")[0], "Dr. Mario");
    }

    #[test]
    fn asset_urls_are_validated() {
        assert_eq!(
            asset_url("games/95324/boxart/IFStolxe.png").as_deref(),
            Some("https://assets.iisu.network/games/95324/boxart/IFStolxe.png")
        );
        assert!(asset_url("../../etc/passwd").is_none());
        assert!(asset_url("/games/1/a.png").is_none());
        assert!(asset_url("https://evil.example/a.png").is_none());
        assert!(asset_url("games/1/a b.png").is_none());
        assert!(asset_url("").is_none());
    }

    #[test]
    fn every_catalog_console_is_mapped_or_explicitly_skipped() {
        // A console added to the catalog without an iiDB mapping would silently
        // never use iiDB. "Multi-System" is the generic RetroArch entry.
        for emu in crate::emulators::get_catalog() {
            if emu.console == "Multi-System" {
                continue;
            }
            assert!(!platforms(&emu.console).is_empty(), "no iiDB platform mapping for console {:?}", emu.console);
        }
    }


}
