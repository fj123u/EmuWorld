# EmuWorld v3 — kit pour coder la refonte de l'interface

Maquettes : https://claude.ai/artifact/LijneYDo8khfAVua97C129
Rien de ce dossier n'est branché dans l'app. C'est le point de départ à reprendre quand la v3 démarre.

## Ce qu'il y a dans ce dossier

| Fichier | Rôle | Étape |
|---|---|---|
| `tokens.css` | Toutes les couleurs, polices, rayons et ombres, en clair et en sombre. Garde les anciens noms de variables comme alias. | 1 |
| `components.css` | Les briques de base : boutons, cartes, pastilles, champs, jaquettes, tuiles, avatars, barre de navigation, onglets. Classes préfixées `v3-`. | 2 |
| `SectionIcon.tsx` | Les 6 icônes à double contour, la barre de navigation `Dock`, et la table qui range les 16 pages dans 6 sections. | 3 |
| `preview.html` | Page de démonstration du kit, avec un bouton clair / sombre. | — |
| `../icon-v3.svg` | L'icône de l'app retenue. | 6 |

Pour voir l'aperçu : lancer `npm run dev`, puis ouvrir http://localhost:1420/scratch/ui-v3/preview.html
(ouvert directement comme fichier, il ne charge pas les CSS).

## Les règles du style

1. **Fond pointillé**, gris très clair (`--bg`) ou presque noir en sombre. Les surfaces sont des panneaux unis (`--panel`), sans transparence ni flou.
2. **Double contour** sur tout ce qui représente quelque chose : icônes de section, jaquettes, tuiles de console, avatars. Classe `.v3-ring`, couleur donnée par `--k`.
3. **La couleur du contour a un sens** : couleur de la section pour la navigation, couleur de la console pour les jeux, accent par défaut. Jamais décorative.
4. **Couleurs douces** : pas d'aplat saturé, pas de contour noir. Un fond coloré est toujours un mélange avec le panneau (`color-mix(in srgb, var(--k) 16%, var(--panel))`).
5. **Le dégradé est rare** : cadre principal (`.v3-frame`, un par écran), bouton d'action (`.v3-btn--go`, un par écran), barres de progression. Nulle part ailleurs.
6. **Titres en Outfit**, tout le reste en Nunito.
7. **Tout est arrondi** : pilules pour les boutons, champs et pastilles ; 24 px pour les cartes.
8. **Cibles ≥ 44 px** de haut pour tout ce qui se clique (souris, tactile, manette).

## État des lieux (relevé le 11/11/2026)

- `src/App.css` : 6996 lignes, **583 couleurs écrites en dur**, dont 43 `color: white` et 102 `rgba(255, 255, 255, …)`. Ce sont elles qui casseront en thème clair : texte blanc sur fond blanc, bordures invisibles.
- `src/App.tsx` : 10723 lignes, 54 couleurs en dur dans des `style={{…}}`, dont 11 textes blancs.
- **5 variables utilisées mais jamais définies** : `--accent` (22 usages), `--border-subtle` (11), `--bg-tertiary` (7), `--card-bg` (7), `--neon-primary` (5). Aujourd'hui elles ne valent rien. `tokens.css` leur donne une valeur : certains éléments vont donc changer d'aspect dès l'étape 1, c'est attendu.
- Navigation : 15 boutons dans `.sidebar__nav` (`App.tsx` vers la ligne 5697), type `Page` ligne 457.
- Tests : `src/App.test.tsx` passe partout par `sidebarButtons()` (ligne 65), qui cherche `.sidebar__nav` et clique sur les libellés « Roms », « Store », « Réglages ».
- Tutoriel : 5 étapes (ligne 10325) qui visent `[data-tour='store' | 'emulators' | 'library' | 'play' | 'friends']`.
- Manette : la liste des éléments navigables est un gros sélecteur ligne 3442 (`.main-content .game-card, … .gamepad-nav-item, … .btn, …`). Le mode Big Picture a le sien (ligne 3229).
- Thèmes : `default`, `midnight`, `oled`, `retro`, `sakura`, `sunset`, posés par `data-theme` sur `<html>` (ligne 1489), plus la teinte perso `data-accent-hue`. Le marketplace enregistre `base_theme` dans Supabase (ligne 8948).

## Correspondance anciennes → nouvelles variables

| Ancienne | Nouvelle | Remarque |
|---|---|---|
| `--bg-primary` | `--bg` | fond de l'app |
| `--bg-secondary`, `--bg-card`, `--card-bg` | `--panel` | une seule surface |
| `--bg-hover`, `--bg-tertiary` | `--soft` | |
| `--bg-active` | `--soft-hover` | |
| `--bg-glass` | `--panel` | plus de verre dépoli |
| `--text-primary` | `--ink` | |
| `--text-secondary` | `--mut` | |
| `--text-muted` | `--faint` | à éviter en petit sur `--soft` |
| `--border`, `--border-subtle` | `--line` | |
| `--border-hover` | `--line-strong` | |
| `--neon-purple`, `--accent-blue`, `--neon-primary` | `--accent` | |
| `--neon-purple-soft` | `--accent-strong` | |
| `--accent-blue-ink` | `--link` | texte coloré lisible sur le panneau |
| `--accent-blue-soft` | `--accent-soft` | |
| `--neon-green` | `--ok` | |
| `--neon-red` | `--danger` | |
| `--neon-amber` | `--sec-trophy` | favoris, étoiles |
| `--neon-pink` | `--sec-store` | |
| `--neon-*-glow` | — | à supprimer : plus de halo lumineux |
| `--volume-primary*` | `--shadow-soft` / `--shadow-lg` | plus de bord 3D |

Les anciens noms restent définis dans `tokens.css` comme alias : on peut migrer page par page et les supprimer à la toute fin.

## Les 16 pages rangées en 6 sections

| Section | Icône | Pages (la première s'ouvre au clic, les autres sont des onglets) |
|---|---|---|
| Accueil | orange | `discover` |
| Jeux | bleu | `library`, `catalog`, `installed` |
| Store | rose | `store`, `marketplace` |
| Social | vert | `friends`, `challenges`, `leaderboard` |
| Succès | jaune | `stats`, `wrap` |
| Réglages | gris | `settings`, `controller`, `backup`, `changelogs`, `account` |

Le type `Page` et `setPage` ne changent pas. La barre surligne `sectionOf(page)` ; les onglets `.v3-tabs` en haut de chaque section appellent `setPage` comme les boutons actuels. La liste des consoles, aujourd'hui sous la barre latérale, passe dans la page Jeux (les tuiles « Tes consoles »).

## Les étapes

Chaque étape est un commit (ou une PR) qui laisse l'app utilisable et `npm run test:all` au vert.

### 1. Jetons de couleur

- Remplacer le bloc `:root` de `App.css` (lignes 9 à 75) par le contenu de `tokens.css`. Remplacer aussi le fond pointillé `body::before` (ligne 219) : il est dans `tokens.css`.
- Thème sombre : `[data-theme="dark"]`. Le thème par défaut devient le clair.
- Chasser les couleurs en dur, dans cet ordre : `color: white` / `#fff` (54 au total), puis `rgba(255, 255, 255, …)` utilisés comme bordure ou fond (→ `--line`, `--soft`), puis le reste au fil des pages. Un texte blanc n'est légitime que sur une image ou sur `--danger`.
- Fait quand : chaque page est lisible en clair **et** en sombre, sans rien redessiner d'autre.
- Test : un test Vitest qui lit `App.css` et échoue si une variable `var(--x)` utilisée n'est définie nulle part (il aurait attrapé les 5 variables fantômes).

### 2. Composants de base

- Copier `components.css` à la fin de `App.css` (ou l'importer depuis `main.tsx`).
- Faire pointer les anciennes classes vers le nouveau style plutôt que de renommer 10 000 lignes de JSX : restyler `.btn`, `.btn--primary`, `.game-card`, `.settings__field-input`, etc. avec les mêmes règles que `.v3-btn`, `.v3-cover`, `.v3-input`. Les classes `v3-` servent au code neuf.
- `.btn` et `.gamepad-nav-item` doivent rester : la navigation manette (ligne 3442) et les règles du projet en dépendent.
- Fait quand : boutons, cartes, champs et cartes de jeu ont le nouveau style partout, sans changement de structure.

### 3. Navigation

- Déplacer `SectionIcon.tsx` dans `src/`, remplacer `<aside className="sidebar">` par `<Dock>` en haut de `.main-content`, ajouter les onglets `.v3-tabs` par section.
- Clés i18n à ajouter dans `fr.ts` et `en.ts` : `nav.home`, `nav.games`, `nav.social`, `nav.trophies` (vérifier celles qui existent déjà, comme `nav.store` ou `nav.settings`).
- Tests : réécrire `sidebarButtons()` pour parcourir la barre **et** les onglets ; le test « ouvre chaque page sans planter » doit toujours visiter les 15 pages. C'est une adaptation au nouveau balisage, pas un affaiblissement : le nombre de pages visitées ne doit pas baisser.
- Tutoriel : reposer les `data-tour` sur les boutons de la barre et sur les onglets. L'étape « emulators » vise un onglet de la section Jeux, qui n'existe que quand cette section est ouverte : le tutoriel change déjà de page avant de chercher l'élément, vérifier que ça suffit.
- Manette : ajouter `.v3-dock__item` et `.v3-tabs__item` au parcours (ils portent déjà `gamepad-nav-item`), et décider comment on passe de la barre au contenu (aujourd'hui : gauche/droite entre barre latérale et contenu ; demain : haut/bas).
- Supprimer `--sidebar-width` et le CSS `.sidebar*` une fois la barre en place.

### 4. Page Accueil

- Aujourd'hui `discover` est l'accueil. La nouvelle page reprend la maquette : cadre « Reprendre la partie », tuiles des consoles, joués récemment, amis en ligne, défi de la semaine, résumé de la semaine.
- Toutes les données existent déjà côté front (playtime, roms, amis, défis) : pas de nouvelle commande Tauri à prévoir.
- Garder ce que `discover` propose aujourd'hui en bas de page plutôt que de le supprimer.

### 5. Pages une par une

Ordre conseillé : Bibliothèque, Social et lobby, Store, Stats, Réglages. Une page par commit, en supprimant au passage les couleurs en dur et les anciens noms de variables de la page.

### 6. Icône de l'app

`npx tauri icon scratch/icon-v3.svg`, puis logo du web panel et asset Discord `emuworld_logo`.

## Points à trancher avant de commencer

1. **Les 5 thèmes actuels.** Ce sont tous des thèmes sombres bâtis sur l'ancienne palette. Le plus simple : les garder comme variantes du thème sombre (ils ne redéfinissent que fonds et accent), et ajouter « Clair » et « Sombre » en tête de liste. Les supprimer casserait les thèmes du marketplace, dont `base_theme` est enregistré dans Supabase.
2. **Thème par défaut.** Clair pour tout le monde, ou clair seulement pour les nouvelles installations en laissant leur thème aux utilisateurs actuels ?
3. **Mode Big Picture.** Il a son propre CSS et sa propre navigation manette. Le refaire dans le même style fait partie de la v3, ou vient après ?
4. **Couleur d'accent perso.** `tokens.css` la garde (teinte adoucie). À vérifier en clair : certaines teintes (jaune) deviennent peu lisibles comme couleur de lien.

## Pièges

- Les règles du projet s'appliquent à chaque étape : textes via `t("…")`, `gamepad-nav-item` sur tout élément interactif, une ligne dans `src/changelog.ts` pour chaque changement visible, tests avant de pousser.
- `color-mix()` et le dégradé en bordure (`.v3-frame`) demandent un moteur récent : WebView2 sous Windows suit Edge, donc ça passe ; à revérifier si le portage Linux / macOS avance.
- Les polices viennent de Google Fonts (comme Nunito aujourd'hui). Hors ligne, Outfit retombe sur Nunito puis sur la police système.
- Le fond des jaquettes vides utilise `--k` : penser à poser `--console-hue` sur chaque carte de jeu (même mécanisme que `data-console-accent`).
