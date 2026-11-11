# EmuWorld — Plan de route

## 🔜 À faire

### 🎨 V3 — Refonte de l'interface

Direction validée le 11/11/2026, à lancer avec la v3. Maquettes : https://claude.ai/artifact/LijneYDo8khfAVua97C129

Style : fond pointillé clair (variante sombre), icônes et jaquettes à double contour (liseré blanc + couleur de la section ou de la console), couleurs douces, dégradé bleu ciel → pervenche → mauve réservé au cadre principal et au bouton d'action, polices Outfit + Nunito.

Kit de départ : `scratch/ui-v3/` — `GUIDE.md` (étapes, correspondance des variables, pièges), `tokens.css`, `components.css`, `SectionIcon.tsx`, `preview.html`.

- [x] **Jetons de couleur** *(fait le 11/11, d4c70c8)* — thème clair par défaut + sombre dans `App.css`, les 5 thèmes actuels deviennent des variantes du sombre proposées par le marketplace, renommer les variables `--neon-*`, corriger les textes clairs codés en dur
- [x] **Composants de base** *(fait le 11/11, 44eb002)* — boutons, cartes, pastilles, champs, cartes de jeu (CSS seul)
- [x] **Navigation** *(fait le 11/11)* — barre d'icônes en pilule : Accueil, Jeux, Store, Social, Succès, Réglages à la place des 15 entrées de la barre latérale (tutoriel `data-tour`, navigation manette et tests à adapter)
- [x] **Page Accueil** *(fait le 11/11 : src/HomePage.tsx + src/home.ts)* — reprendre la partie, consoles, joués récemment, amis en ligne, défi de la semaine
- [x] **Pages une par une** *(fait le 11/11 : Bibliothèque, Social + page Lobbies, Store, Stats, Réglages)* — Bibliothèque, Social et lobby, puis Store, Stats, Réglages
- [x] **Mode Big Picture** *(fait le 11/11 ; la barre à 6 sections n'y est pas : Big Picture ne montre que la bibliothèque)* — refait dans le même style (tailles télé, focus manette)
- [x] **Icône de l'app** *(fait le 11/11 ; reste à téléverser scratch/emuworld_logo_1024.png comme asset Discord `emuworld_logo`)* — console claire à boutons colorés (choisie le 11/11/2026), source dans `scratch/icon-v3.svg` ; régénérer `src-tauri/icons/` avec `npx tauri icon`, puis mettre à jour le logo du web panel et l'asset Discord `emuworld_logo`

### 🕹️ Multijoueur

- [x] **Lobbies en mode Big Picture** — Y crée un lobby, Start passe dans le panneau (2.7.4)
- [ ] **Tournois** *(gros)* — brackets entre amis (simple/double élimination), seeds et résultats
- [ ] **Clubs / Guildes** *(gros)* — groupes permanents avec chat de groupe, événements, classement interne
- [ ] **Live spectate** *(très gros)* — regarder un ami jouer en direct (basse qualité mais instantané)

### 🎮 Gameplay

- [ ] **Cheats database** *(moyen)* — codes Action Replay / GameShark activables en un clic avant de lancer le jeu
- [ ] **Input display / macros** *(moyen)* — inputs en overlay (streams), enregistrer et rejouer des macros
- [ ] **Auto-config émulateurs** *(moyen)* — détecter GPU/CPU et régler chaque émulateur automatiquement
- [ ] **Performance monitoring** *(quick win)* — FPS, latence, RAM/CPU dans l'overlay

### 👥 Social & profil

- [ ] **Profil personnalisable** *(quick win)* — bannière, bio, jeu favori épinglé, badges mis en avant sur le profil public
- [ ] **Système de parrainage** *(quick win)* — code d'invitation, badges quand un ami s'inscrit
- [ ] **Galerie de screenshots améliorée** *(moyen)* — filtres, partage Discord/amis, comparaison, slideshow
- [ ] **Trading cards** *(moyen)* — cartes à collectionner par jeu/succès, échangeables entre amis
- [ ] **Game recommendations AI** *(gros)* — recommandations selon les habitudes de jeu (temps, genre, console)
- [ ] **Widgets bureau** *(moyen)* — dernière session, jeu en cours, succès récent, série

### 📱 Multi-plateforme & cloud

- [ ] **Synchronisation multi-PC** *(moyen)* — config, playtime et saves entre plusieurs installations via Supabase
- [ ] **Web app** *(gros)* — bibliothèque, stats et amis consultables depuis un navigateur (sans émulation)
- [ ] **Linux / macOS** *(gros)* — Tauri est multi-plateforme, mais les émulateurs changent
- [ ] **App mobile compagnon** *(très gros)* — bibliothèque, stats, succès, chat amis, lancement à distance
- [ ] **Cloud gaming** *(très gros)* — streamer un jeu du PC vers un autre appareil (Moonlight/Sunshine)

### 🔧 Technique

- [ ] **Versionner le schéma Supabase** *(P2)* — convertir les scripts SQL manuels de `scratch/` en migrations ordonnées et rejouables, tester les politiques RLS et les contraintes de chaque table avant déploiement
- [ ] **Découper les deux monolithes** *(P2)* — `src/App.tsx` et `src-tauri/src/lib.rs` : extraire progressivement les domaines (bibliothèque, social, synchronisation, commandes Tauri) sans changer le comportement
- [ ] **Plugin system** *(très gros)* — API d'extensions (nouveaux émulateurs, sources de covers, intégrations)

## ✅ Fait

**Base** — horloge, Discord Rich Presence, navigation manette complète (clavier virtuel, remap), mise à jour automatique, installeur signé par GitHub Actions, onboarding, multi-langue FR/EN, logs & diagnostic, mode portable, import/export de config, raccourcis globaux, notifications système, limiteur de bande passante.

**Bibliothèque** — vue grille/liste, tri et filtres, collections, smart playlists, notes et étoiles par jeu, détection auto des ROMs, ROM health check, covers (cascade de sources), fond dynamique, animation de lancement, thèmes + couleur d'accent, thème par console, mode Big Picture.

**Jeu** — overlay Shift+Tab (succès, chat, notes), screenshots, récap de session, speedrun timer, gestion des save states (partage : `scratch/supabase_savestates_sharing.sql`), rewind RetroArch, mise à jour auto des émulateurs, guides intégrés.

**Succès & stats** — succès internes avec animation de déblocage, RetroAchievements par jeu, statistiques détaillées, Wrap mensuel, classement.

**Social** — amis (présence, chat 1:1, invitations), lobbies multijoueur (netplay, chat de lobby, LDN Switch), fil d'activité, comparaison avec un ami, défis hebdo et versus, reviews communautaires, recommandations, marketplace de thèmes, page Discover, profil public partageable.

**Cloud & web** — sauvegarde cloud des saves, web panel sur GitHub Pages (alwaysdata supprimé le 05/11/2026).

**Qualité** — données locales protégées contre la corruption (écriture atomique), dépendances auditées, tests automatisés et CI sur chaque push. Poids du JS au démarrage mesuré le 06/11/2026 : découpage jugé inutile (lecture disque, 18–31 ms).

**Bugs corrigés** — covers manquantes (dont Wii U), OAuth Google/Discord et chargement infini à la connexion, lancement NES, lancement de ROM dans Cemu (`-g`), extraction des ROMs Wii/GameCube, covers du site web, navigation manette.

> Discord Rich Presence : l'ID d'application par défaut est dans `src-tauri/src/discord_rpc.rs` ; les assets à uploader sur le Developer Portal sont `emuworld_logo`, `playing_generic` et les `console_*` listés par `console_icon_key`.
