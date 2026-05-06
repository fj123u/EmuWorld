use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub unlocked: bool,
    pub unlocked_at: Option<String>,
    pub hidden: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AchievementsStore {
    #[serde(default)]
    pub unlocked: HashMap<String, String>, // id → unlocked_at (RFC3339)
}

#[derive(Debug, Serialize, Clone)]
pub struct AchievementDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub hidden: bool,
}

pub fn all_achievements() -> Vec<AchievementDef> {
    vec![
        // ====== Milestones (visible) ======
        // Library
        AchievementDef { id: "library_10", name: "Collectionneur", description: "10 jeux dans la bibliothèque", icon: "📚", hidden: false },
        AchievementDef { id: "library_50", name: "Bibliophile", description: "50 jeux dans la bibliothèque", icon: "🏛️", hidden: false },
        AchievementDef { id: "library_100", name: "Archiviste", description: "100 jeux dans la bibliothèque", icon: "🗄️", hidden: false },
        AchievementDef { id: "library_500", name: "Musée vivant", description: "500 jeux dans la bibliothèque", icon: "🏆", hidden: false },
        // Playtime
        AchievementDef { id: "playtime_1h", name: "Première heure", description: "1 heure de jeu au total", icon: "⏱️", hidden: false },
        AchievementDef { id: "playtime_10h", name: "Joueur assidu", description: "10 heures de jeu au total", icon: "🎮", hidden: false },
        AchievementDef { id: "playtime_50h", name: "Vétéran", description: "50 heures de jeu au total", icon: "🎖️", hidden: false },
        AchievementDef { id: "playtime_100h", name: "Hardcore", description: "100 heures de jeu au total", icon: "💎", hidden: false },
        AchievementDef { id: "playtime_500h", name: "Légende", description: "500 heures de jeu au total", icon: "👑", hidden: false },
        // Launches
        AchievementDef { id: "launches_10", name: "Habitué", description: "10 sessions de jeu", icon: "🚀", hidden: false },
        AchievementDef { id: "launches_50", name: "Régulier", description: "50 sessions de jeu", icon: "🔥", hidden: false },
        AchievementDef { id: "launches_100", name: "Infatigable", description: "100 sessions de jeu", icon: "⚡", hidden: false },
        AchievementDef { id: "launches_500", name: "Machine", description: "500 sessions de jeu", icon: "🤖", hidden: false },
        // Consoles
        AchievementDef { id: "consoles_3", name: "Curieux", description: "3 consoles différentes jouées", icon: "🕹️", hidden: false },
        AchievementDef { id: "consoles_5", name: "Multi-plateformes", description: "5 consoles différentes jouées", icon: "🌐", hidden: false },
        AchievementDef { id: "consoles_10", name: "Omni-gamer", description: "10 consoles différentes jouées", icon: "🌟", hidden: false },
        // Emulators
        AchievementDef { id: "emulators_3", name: "Équipé", description: "3 émulateurs installés", icon: "🔧", hidden: false },
        AchievementDef { id: "emulators_5", name: "Arsenal", description: "5 émulateurs installés", icon: "🛠️", hidden: false },
        // Streak
        AchievementDef { id: "streak_3", name: "Trilogie", description: "3 jours de jeu consécutifs", icon: "📅", hidden: false },
        AchievementDef { id: "streak_7", name: "Semaine parfaite", description: "7 jours de jeu consécutifs", icon: "🗓️", hidden: false },
        AchievementDef { id: "streak_30", name: "Mois de feu", description: "30 jours de jeu consécutifs", icon: "🔥", hidden: false },

        // ====== Hidden achievements (cachés — revealed only once unlocked) ======
        AchievementDef { id: "first_favorite", name: "Coup de cœur", description: "Mettre un jeu en favori", icon: "❤️", hidden: true },
        AchievementDef { id: "first_download", name: "Première prise", description: "Télécharger un jeu depuis le store", icon: "⬇️", hidden: true },
        AchievementDef { id: "login_discord", name: "Discord Gang", description: "Se connecter avec Discord", icon: "🟣", hidden: true },
        AchievementDef { id: "login_google", name: "Googler", description: "Se connecter avec Google", icon: "🔵", hidden: true },
        AchievementDef { id: "change_roms_dir", name: "À ma façon", description: "Changer l'emplacement des ROMs", icon: "📂", hidden: true },
        AchievementDef { id: "change_avatar", name: "Nouveau look", description: "Changer sa photo de profil", icon: "📸", hidden: true },
        AchievementDef { id: "night_owl", name: "Oiseau de nuit", description: "Lancer un jeu entre 2h et 5h du matin", icon: "🦉", hidden: true },
        AchievementDef { id: "speed_runner", name: "Speed Runner", description: "Lancer et fermer un jeu en moins de 30 secondes", icon: "⚡", hidden: true },
        AchievementDef { id: "marathon", name: "Marathon", description: "Session de jeu de plus de 4 heures d'affilée", icon: "🏃", hidden: true },
        AchievementDef { id: "install_update", name: "À jour !", description: "Installer une mise à jour de EmuWorld", icon: "✨", hidden: true },
        AchievementDef { id: "ten_favorites", name: "Fan absolu", description: "Avoir 10 jeux en favoris", icon: "💕", hidden: true },
        AchievementDef { id: "clear_covers", name: "Ménage de printemps", description: "Vider le cache des covers", icon: "🧹", hidden: true },
        AchievementDef { id: "ra_connected", name: "Chasseur de trophées", description: "Connecter son compte RetroAchievements", icon: "🏆", hidden: true },
        AchievementDef { id: "ra_first_100", name: "Perfectionniste", description: "Avoir un jeu à 100% sur RetroAchievements", icon: "💯", hidden: true },
        AchievementDef { id: "ra_five_100", name: "Complétionniste", description: "Avoir 5 jeux à 100% sur RetroAchievements", icon: "🌟", hidden: true },
    ]
}

fn store_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("EmuWorld");
    let _ = std::fs::create_dir_all(&path);
    path.push("achievements.json");
    path
}

pub fn load() -> AchievementsStore {
    let path = store_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str::<AchievementsStore>(&data) {
            return store;
        }
    }
    AchievementsStore::default()
}

pub fn save(store: &AchievementsStore) -> Result<(), String> {
    let path = store_path();
    let data = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn get_all_with_status() -> Vec<Achievement> {
    let store = load();
    let defs = all_achievements();
    defs.iter().map(|def| {
        let unlocked_at = store.unlocked.get(def.id).cloned();
        Achievement {
            id: def.id.to_string(),
            name: def.name.to_string(),
            description: def.description.to_string(),
            icon: def.icon.to_string(),
            unlocked: unlocked_at.is_some(),
            unlocked_at,
            hidden: def.hidden,
        }
    }).collect()
}

pub fn unlocked_count() -> u32 {
    load().unlocked.len() as u32
}

pub fn rank_label(count: u32) -> &'static str {
    match count {
        0..=2 => "Bronze",
        3..=5 => "Argent",
        6..=9 => "Or",
        10..=14 => "Platine",
        _ => "Diamant",
    }
}

pub fn rank_icon(count: u32) -> &'static str {
    match count {
        0..=2 => "🥉",
        3..=5 => "🥈",
        6..=9 => "🥇",
        10..=14 => "💠",
        _ => "💎",
    }
}

/// Unlock a single achievement by id (for event-driven hidden achievements).
/// Returns the Achievement if newly unlocked, None if already had it.
pub fn unlock_single(id: &str) -> Option<Achievement> {
    let mut store = load();
    if store.unlocked.contains_key(id) {
        return None;
    }
    let defs = all_achievements();
    let def = defs.iter().find(|d| d.id == id)?;
    let now = chrono::Utc::now().to_rfc3339();
    store.unlocked.insert(id.to_string(), now.clone());
    let _ = save(&store);
    Some(Achievement {
        id: def.id.to_string(),
        name: def.name.to_string(),
        description: def.description.to_string(),
        icon: def.icon.to_string(),
        unlocked: true,
        unlocked_at: Some(now),
        hidden: def.hidden,
    })
}

/// Check milestone achievements based on current stats. Returns newly unlocked IDs.
pub fn check_and_unlock(
    library_count: u32,
    total_seconds: u64,
    total_launches: u32,
    consoles_played: u32,
    favorite_count: u32,
    emulators_installed: u32,
    streak_days: u32,
    has_downloaded: bool,
) -> Vec<Achievement> {
    let mut store = load();
    let now = chrono::Utc::now().to_rfc3339();
    let mut newly_unlocked = Vec::new();

    let checks: Vec<(&str, bool)> = vec![
        ("library_10", library_count >= 10),
        ("library_50", library_count >= 50),
        ("library_100", library_count >= 100),
        ("library_500", library_count >= 500),
        ("playtime_1h", total_seconds >= 3600),
        ("playtime_10h", total_seconds >= 36000),
        ("playtime_50h", total_seconds >= 180000),
        ("playtime_100h", total_seconds >= 360000),
        ("playtime_500h", total_seconds >= 1800000),
        ("launches_10", total_launches >= 10),
        ("launches_50", total_launches >= 50),
        ("launches_100", total_launches >= 100),
        ("launches_500", total_launches >= 500),
        ("consoles_3", consoles_played >= 3),
        ("consoles_5", consoles_played >= 5),
        ("consoles_10", consoles_played >= 10),
        ("first_favorite", favorite_count >= 1),
        ("ten_favorites", favorite_count >= 10),
        ("emulators_3", emulators_installed >= 3),
        ("emulators_5", emulators_installed >= 5),
        ("streak_3", streak_days >= 3),
        ("streak_7", streak_days >= 7),
        ("streak_30", streak_days >= 30),
        ("first_download", has_downloaded),
    ];

    let defs = all_achievements();
    for (id, condition) in checks {
        if condition && !store.unlocked.contains_key(id) {
            store.unlocked.insert(id.to_string(), now.clone());
            if let Some(def) = defs.iter().find(|d| d.id == id) {
                newly_unlocked.push(Achievement {
                    id: def.id.to_string(),
                    name: def.name.to_string(),
                    description: def.description.to_string(),
                    icon: def.icon.to_string(),
                    unlocked: true,
                    unlocked_at: Some(now.clone()),
                    hidden: def.hidden,
                });
            }
        }
    }

    if !newly_unlocked.is_empty() {
        let _ = save(&store);
    }

    newly_unlocked
}
