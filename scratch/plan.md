# EmuWorld — Plan de route

## 🚀 Ajouts

- [x] **Date / heure dans l'app** *(quick win ~10 min)* — horloge discrète dans la titlebar ou sidebar footer
- [x] **Discord Rich Presence** *(moyen)* — "Jouant à Mario Kart 8 via EmuWorld" avec le logo EmuWorld en grand (brand persistant) et l'icône de la console en petit. Idle = "Browsing the library · In the launcher"

  > ⚠️ Configuration requise côté Discord Developer Portal :
  > 1. App actuelle : ID par défaut `1334862011723284510` dans `src-tauri/src/discord_rpc.rs` — remplace par le tien si tu veux être propriétaire
  > 2. Uploader les assets avec ces keys exactes : `emuworld_logo` (grand), `playing_generic` (petit fallback), et `console_switch` / `console_wiiu` / `console_ps2` etc. (voir `console_icon_key` dans le module Rust pour la liste)
  > 3. Sans assets, les icônes ne s'affichent pas mais le texte lui fonctionne quand même.
- [x] **Manette (navigation UI au pad)** *(gros)* — Gilrs natif Rust, highlight focus, D-pad/stick → navigate, A → click/context menu, B → back, clavier virtuel, remap touches
- [x] **Achievements in-app** *(gros)* — palier par installation, nb de ROMs, heures jouées, jeu favori, 10 consoles, etc. Notif in-app + badges sur le profil web
- [x] **RetroAchievements par jeu** *(gros)* — wrapper de l'API [retroachievements.org](https://retroachievements.org), rattachement du compte, progression affichée sur la fiche jeu, auto-configuration des émulateurs (token injection + cores RetroArch), redirection automatique vers RetroArch pour les standalone sans support RA
- [x] **Executable partageable** — workflow GitHub Actions qui build + signe le `.exe`/`.msi` à chaque tag `v*`, release en prerelease, guide dans [scratch/release_guide.md](scratch/release_guide.md)
- [x] **Système d'amis** *(hard)*
  - [x] liste d'amis (table Supabase `friendships` avec statut pending/accepted)
  - [x] présence : "x est en train de jouer à Y"
  - [ ] inviter dans un lobby
  - [x] voir le profil
  - [ ] chat 1:1
- [ ] **Multijoueur / lobby** *(très hard, dépend des émulateurs)* — netplay natif supporté uniquement par certains émus (RetroArch, Dolphin, Citra). On pourrait wrapper ces features.
- [x] **Update Automatique** — plugin Tauri updater + signing key + workflow GitHub Actions. L'app check `latest.json` au boot, affiche un bouton "✨ Mise à jour" dans la titlebar, download + relance avec progress. Guide dans [scratch/release_guide.md](scratch/release_guide.md).
- [x] **leaderboard in app**
- [x] **Backup cloud des saves** — *Techniquement faisable sans souci* : scanner le dossier saves de chaque émulateur connu, zipper par jeu, uploader sur Supabase Storage (~quelques MB par save). Versioning simple avec timestamp. **Limite** : le plan gratuit Supabase c'est 1 GB de Storage donc ça tient facilement tant que tu restes seul. Si tu veux pousser à des amis c'est peut-être 5 GB (~50 jeux × 100 MB) qui suffisent aussi. Pas lourd, juste chiadé côté UI.

## 🐛 Bugs

- [x] **Covers qui restent manquantes** sur certains jeux (lesquels déjà ? faudrait lister les noms) et bugs sur covers wii u
- [x] **OAuth Google / Discord** — `register_all()` au boot, page bounce sur alwaysdata pour que l'onglet se ferme proprement, playtime réinitialisé au switch de compte, et fix du listener `game-closed` qui tournait dans le vide.
- [x] **Lancement jeux NES** ne fonctionne pas — à investiguer : quel émulateur est mappé au NES, quelle erreur remonte ?
- [ ] **Chargement Infini** page de connexion — page bounce `/auth-callback.html` affiche un message "tu peux fermer cette fenêtre".
- [x] **Probleme lancement Cemu** lors du lancement d'une rom, cemu se lance bien masi pas la rom → fix: ajout flag `-g`
- [x] **Probleme installation roms wii/gamecube** lors du finalize, la rom n'est pas dezip et dolphin n'arrive pas a lire le .ciso → fix: extraction 7z séparée du ZIP + ajout extensions ciso/gcz/nkit.iso
- [x] **Probleme covers sites web**
- [x] **Rendre tout controlable par manette**
- [ ] **Mannette pas parfaite**

## 💡 Idées en vrac (à rajouter au fil de l'eau)

- [x] Session recap toast à la fermeture de l'émulateur ("Session de 2h34 · +3 launches · total 47h")
- [ ] Discover page avec jeu du jour, top friends, covers en parallax
- [ ] Mode "Versus" / challenge 7 jours entre amis

### 🎨 UI / UX

- [x] **Thèmes personnalisables** *(moyen)* — 5-6 thèmes prédéfinis (dark purple actuel, midnight blue, OLED black, retro green CRT, pastel light) + accent color picker custom
- [x] **Animations de lancement** *(quick win)* — quand tu lances un jeu, splash animé avec le logo console + nom du jeu pendant 2-3s (style boot screen)
- [x] **Vue en liste / grille toggle** *(quick win)* — switcher entre la grille de covers actuelle et une vue tableau (nom, console, temps joué, dernier lancement, note)
- [x] **Tri et filtres avancés** *(moyen)* — trier par : temps joué, dernier lancé, alphabétique, note perso, nb de launches. Filtrer par : non-joués, favoris, complétés
- [x] **Catégories / Collections custom** *(moyen)* — l'utilisateur crée ses propres playlists de jeux ("RPGs du moment", "Couch co-op", "Backlog", "Terminés")
- [ ] **Mode Fullscreen / Big Picture** *(gros)* — UI alternative optimisée TV avec grosses covers, navigation 100% manette, pas de sidebar, style Steam Big Picture
- [x] **Fond d'écran dynamique** *(quick win)* — utiliser la cover du jeu sélectionné en background blur derrière le contenu principal (style PlayStation Store)
- [ ] **Onboarding première utilisation** *(moyen)* — wizard de setup : choisir émulateurs, scanner dossier ROMs, connecter compte, tuto manette. Sinon l'app est intimidante au first launch

### 📊 Statistiques & Social

- [x] **Statistiques détaillées** *(moyen)* — page stats perso : graphe d'heures jouées par semaine (sparkline), top 5 jeux, top 3 consoles, streak actuel (jours consécutifs), heatmap style GitHub des sessions
- [ ] **Wrap annuel / mensuel** *(gros)* — "EmuWorld Wrapped" à la Spotify : tes stats du mois en slides animées (jeu le + joué, console préférée, temps total, achievement débloqués)
- [ ] **Partage de profil** *(moyen)* — URL publique `emuworld.app/u/username` avec stats, bibliothèque, achievements, top jeux (la web-panel actuelle enrichie)
- [ ] **Activity feed** *(gros)* — fil d'activité entre amis : "Théo a joué 3h à Zelda", "Max a débloqué un achievement", "Léo a ajouté 12 jeux à sa bibliothèque"
- [ ] **Comparer avec un ami** *(moyen)* — vue side-by-side des stats entre toi et un pote (qui joue plus à quoi, qui a plus d'achievements, etc.)

### 🖥️ Mode Intégré (Killer Feature)

- [ ] **Game View embarquée** *(très gros)* — l'émulateur s'affiche directement dans EmuWorld comme un panel central, avec des onglets latéraux rétractables (achievements, chat, notes, timer, guide). Navigation style Twitch/YouTube. Deux approches techniques possibles :
  - **Option A — Win32 `SetParent`** : reparenter le HWND de l'émulateur dans un conteneur Tauri. Plus intégré (1 seule fenêtre), mais certains émus gèrent mal.
  - **Option B — Overlay transparent** : fenêtre EmuWorld always-on-top semi-transparente par-dessus l'émulateur, panels rétractables sur les côtés. Plus compatible, 2 fenêtres.
  - Panels possibles : RetroAchievements live, chat amis, notes/codes, speedrun timer, guides, screenshots
  - Toggle avec un raccourci (ex: F1 ou bouton manette) pour montrer/cacher les panels

### 🎮 Gameplay & Émulation

- [ ] **Screenshare / Screenshots** *(moyen)* — capturer un screenshot depuis l'émulateur en un raccourci, galerie intégrée dans la fiche du jeu
- [x] **Notes par jeu** *(quick win)* — champ texte libre sur chaque fiche jeu pour noter des codes, astuces, où on en est, etc.
- [x] **Rating / Note perso** *(quick win)* — étoiles ou note /10 sur chaque jeu, visible dans la bibliothèque et utilisable comme filtre/tri
- [ ] **Détection auto des ROMs** *(moyen)* — watcher sur le dossier ROMs qui détecte les nouveaux fichiers et propose automatiquement de les ajouter à la bibliothèque sans rescan manuel
- [ ] **Cheat codes database** *(moyen)* — intégrer une base de cheats (Action Replay, GameShark) par jeu, activables en un clic avant le lancement
- [ ] **Speed run timer** *(gros)* — chrono intégré avec splits, comparable avec tes propres records et éventuellement ceux des amis (comme livesplit)

### 🔧 Technique & QoL

- [x] **Import/Export de config** *(quick win)* — exporter toute sa config EmuWorld (émulateurs, paths, préférences) en JSON pour la restaurer sur un autre PC
- [x] **Raccourcis clavier globaux** *(quick win)* — hotkeys pour lancer le dernier jeu joué, ouvrir EmuWorld, kill l'émulateur en cours, etc. même quand l'app est minimisée
- [ ] **Multi-langue** *(moyen)* — i18n avec au minimum FR/EN, fichier JSON de traductions, détection locale système
- [ ] **Logs & diagnostic** *(quick win)* — page dans settings qui affiche les derniers logs (lancement ému, fetch cover, erreurs) pour debug sans console
- [ ] **Portable mode** *(moyen)* — détection d'un fichier `portable.txt` à côté de l'exe → stocke tout dans le même dossier (USB friendly)
- [ ] **Bandwidth limiter** *(quick win)* — réglage vitesse max de download dans le store (pour pas saturer la connexion)
- [x] **Notifications système** *(quick win)* — notification Windows native quand un download est terminé, quand un ami se connecte, quand un achievement est débloqué

### 🌐 Communauté & Contenu

- [ ] **Guides / Walkthroughs intégrés** *(gros)* — pour chaque jeu, panel latéral avec des guides texte/image scrapés depuis des wikis ou écrits par la communauté
- [ ] **Recommandations "Si t'as aimé X..."** *(moyen)* — algo simple basé sur console + genre + tags pour suggérer des jeux similaires depuis le store
- [ ] **Reviews par la communauté** *(gros)* — notes + commentaires publics sur chaque jeu, visibles par tous les utilisateurs EmuWorld
- [ ] **Événements / Challenges hebdo** *(gros)* — "Cette semaine : finir Mega Man 2 en moins de 2h" avec leaderboard dédié et badge reward
- [ ] **Marketplace de thèmes** *(moyen)* — les utilisateurs partagent leurs thèmes custom, téléchargeables en un clic depuis un onglet "Community"
