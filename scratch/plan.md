# EmuWorld — Plan de route

## 🚀 Ajouts

- [x] **Date / heure dans l'app** *(quick win ~10 min)* — horloge discrète dans la titlebar ou sidebar footer
- [ ] **Discord Rich Presence** *(moyen)* — "Jouant à Mario Kart 8 • 14 h lifetime" avec cover en icône
- [ ] **Manette (navigation UI au pad)** *(gros)* — Gamepad API, highlight focus, D-pad → navigate, A → click, B → back
- [ ] **Achievements in-app** *(gros)* — palier par installation, nb de ROMs, heures jouées, jeu favori, 10 consoles, etc. Notif in-app + badges sur le profil web
- [ ] **RetroAchievements par jeu** *(gros)* — wrapper de l'API [retroachievements.org](https://retroachievements.org), rattachement du compte, progression affichée sur la fiche jeu
- [ ] **Executable partageable** *(trivial)* — `npm run tauri build` produit un `.msi`/`.exe` dans `src-tauri/target/release/bundle/`. À documenter + signer si possible
- [ ] **Système d'amis** *(hard)*
  - [ ] liste d'amis (table Supabase `friendships` avec statut pending/accepted)
  - [ ] présence : "x est en train de jouer à Y"
  - [ ] inviter dans un lobby
  - [ ] voir le profil
  - [ ] chat 1:1
- [ ] **Multijoueur / lobby** *(très hard, dépend des émulateurs)* — netplay natif supporté uniquement par certains émus (RetroArch, Dolphin, Citra). On pourrait wrapper ces features.

### À trancher
- [ ] **Backup cloud des saves** — *Techniquement faisable sans souci* : scanner le dossier saves de chaque émulateur connu, zipper par jeu, uploader sur Supabase Storage (~quelques MB par save). Versioning simple avec timestamp. **Limite** : le plan gratuit Supabase c'est 1 GB de Storage donc ça tient facilement tant que tu restes seul. Si tu veux pousser à des amis c'est peut-être 5 GB (~50 jeux × 100 MB) qui suffisent aussi. Pas lourd, juste chiadé côté UI.

## 🐛 Bugs

- [ ] **Covers qui restent manquantes** sur certains jeux (lesquels déjà ? faudrait lister les noms)
- [ ] **OAuth Google / Discord / GitHub** — probablement un souci de redirect URL dans Supabase + config dans `src-tauri/tauri.conf.json` deep-link
- [ ] **Lancement jeux NES** ne fonctionne pas — à investiguer : quel émulateur est mappé au NES, quelle erreur remonte ?

## 💡 Idées en vrac (à rajouter au fil de l'eau)

- [ ] Session recap toast à la fermeture de l'émulateur ("Session de 2h34 · +3 launches · total 47h")
- [ ] Discover page avec jeu du jour, top friends, covers en parallax
- [ ] Mode "Versus" / challenge 7 jours entre amis
