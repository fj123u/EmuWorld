# Switch 2 dans EmuWorld avec Lichii — note de recherche

Relevé le 11/11/2026 sur https://github.com/zortos293/Lichii (branche `nx2`, build 150). Rien n'est codé.
Le projet a 11 jours et sort un à deux builds par jour : tout ce qui suit peut changer.

**Fait le 12/11/2026** (build 160) : entrée `lichii` dans le catalogue, console « Nintendo Switch 2 », extension `nup`, lancement `-g <jeu> -f`, mises à jour suivies. Écart avec la note : **pas de mode portable**. Une mise à jour d'émulateur par EmuWorld supprime puis réinstalle tout son dossier, donc un dossier `user/` à l'intérieur ferait perdre les sauvegardes à chaque build ; Lichii garde ses données dans `%APPDATA%\lichii`. Les points « à vérifier » ci-dessous restent à tester avec un vrai jeu.

## Ce qu'est Lichii

- Émulateur Switch **et** Switch 2, en C++, GPL-3.0. C'est un fork d'Eden (lui-même issu de yuzu), via Eglantine et Sudachi. L'interface, la ligne de commande et les dossiers sont donc ceux de yuzu/Eden.
- Créé le 31/10/2026. 8 200 étoiles, 635 tickets ouverts, 150 builds en 11 jours. Le dépôt se décrit comme « VERY experimental ».
- Téléchargement **uniquement** depuis les releases GitHub : le README signale de faux sites (lichii.org) qui distribuent un malware.
- Seul site officiel : la base de compatibilité https://lichii.zortos.me.

## Ce qu'il faut savoir pour l'intégrer

| Sujet | Constat | Source |
|---|---|---|
| Exécutable Windows | `lichii.exe`, dans un zip à extraire en entier | README |
| Archives Windows | `lichii-windows-x86_64.zip` et `lichii-windows-x86_64-pgo.zip` (~90 Mo), noms stables d'un build à l'autre | releases |
| Tags | `build-150`, `build-130`… (pas de numéro de version) | releases |
| Lancer un jeu | `lichii.exe -f -g "<chemin>"` (`-f` plein écran, `-g` jeu) | `docs/user/CommandLine.md` |
| Format `.nup` | « NX Package » : un seul fichier avec le jeu de base, ses mises à jour et ses DLC, en arborescences ExeFS/RomFS déjà déchiffrées, compressées en zstd. Magic `NUP\x01`. | `src/core/file_sys/nup_container.h` |
| Métadonnées dans le `.nup` | Title ID, nom du jeu (et noms localisés), éditeur, **icône**, version affichée | idem |
| Autres formats | `nsp`, `xci`, `nsz`, `xcz`, `nca`, `nro`, `nx` | `src/core/loader/loader.cpp` |
| Title ID Switch 2 | commencent par `04…` (ex. `0400486024DEC000`), contre `01…` sur Switch | `NX2_NOTES.md`, tickets |
| Données utilisateur | `%APPDATA%\lichii` ; un dossier `user/` à côté de l'exe est prioritaire (mode portable) | README |
| Premier lancement | assistant d'import des profils Eden, seulement en mode non portable | README |
| Lancement par un frontend | depuis le build 150, plus de fenêtre « réglages recommandés » au démarrage d'un jeu | notes du build 150 |
| Clés et firmware | les notes de dev lancent un jeu « no keys or firmware installed » ; un ticket ouvert (#803) parle de clés requises pour certains contenus | `NX2_NOTES.md`, #803 |

## Comment ça rentrerait dans EmuWorld

Le catalogue est fait pour ça : une entrée dans `emulators.rs`, quelques `match` dans `lib.rs`.

1. **Catalogue** (`emulators.rs`) : nouvelle entrée `lichii`, console **« Nintendo Switch 2 »**, `executable_name: "lichii.exe"`, `archive_type: "zip"`, `supported_extensions: ["nup"]`. URL stable : `https://github.com/zortos293/Lichii/releases/latest/download/lichii-windows-x86_64-pgo.zip`.
   - Ne déclarer que `nup` : `nsp` et `xci` restent à Ryubing, sinon les jeux Switch changeraient d'émulateur.
   - `setup_files` vide : pas de clés ni de firmware téléchargés automatiquement.
2. **Mode portable** : créer un dossier `user/` dans le dossier d'installation juste après l'extraction. Les réglages et les sauvegardes restent dans `%LOCALAPPDATA%/EmuWorld/Emulators/lichii/`, et l'assistant d'import ne s'affiche pas.
3. **Lancement** (`lib.rs` ~1222 et ~1244) : ajouter `-g` avant le chemin comme pour Cemu, et `-f` pour le plein écran.
4. **Mises à jour** (`update_info`) : `UpdateSource::GitHub("zortos293/Lichii", "build-150")`. Le code actuel prend le premier asset dont le nom contient `win` et `x86_64` : ce sera la variante `-pgo` (ordre alphabétique), la plus rapide et la plus téléchargée. Avec un à deux builds par jour, l'app proposera une mise à jour presque à chaque ouverture.
5. **Console** : « Nintendo Switch 2 » est une nouvelle console partout où il y a une liste — alias de dossier (`lib.rs` ~1657 : `switch 2`, `switch2`, `ns2`), `launchbox::platforms` (un test l'exige), `iidb_api`, noms libretro, couleur de console, icône Discord `console_*`.
6. **Scan** : un `.nup` contient base, mises à jour et DLC dans un seul fichier. Le filtre « update ou DLC » par Title ID de la Switch ne s'applique pas.
7. **Jaquettes** : nlib.cc et tinfoil sont des bases Switch 1. Restent LaunchBox et iiDB si leur plateforme Switch 2 existe (à vérifier), puis Wikipedia. Piste propre à ce format : lire l'icône et le titre directement dans le `.nup`.

## À vérifier avant de coder (il faut un build installé)

- La structure du zip Windows (dossier racine ou non) et la présence de `lichii.exe` à la racine.
- Que `user/` à côté de l'exe suffit bien pour le mode portable sur ce fork, et qu'une mise à jour par EmuWorld ne l'écrase pas. Un ticket ouvert (#1287) se plaint de réglages perdus à chaque mise à jour.
- Qu'un `.nup` démarre sans clés ni firmware avec `-g`, et ce que l'émulateur affiche s'il en manque.
- Que `-f -g` ouvre directement le jeu sans fenêtre intermédiaire (build ≥ 150).
- Que la fermeture du jeu termine bien le processus : le suivi du temps de jeu en dépend.
- Les plateformes Switch 2 de LaunchBox et d'iiDB.

## Risques

- **Instabilité** : format `.nup` et options peuvent changer d'un jour à l'autre. Afficher l'émulateur comme expérimental dans le catalogue.
- **Matériel** : émulation lourde, DLSS/FSR mis en avant, tickets de mémoire vidéo saturée à 16 Go.
- **Compatibilité** : très peu de jeux tournent correctement (Ocarina of Time, Mario Kart World et Donkey Kong Bananza concentrent les correctifs).
- **Console en vente** : ne pas ajouter de source de jeux au store ni de téléchargement de clés pour la Switch 2.
- **Faux sites** : ne jamais télécharger ailleurs que sur les releases GitHub.
