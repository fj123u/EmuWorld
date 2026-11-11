# Refonte des défis — plan

Écrit le 12/11/2026. Rien n'est codé. Les choix marqués « à trancher » ont une valeur par défaut : le plan tient debout sans réponse, mais dis-moi si tu veux autre chose.

## Le but

Des défis qui avancent tout seuls quand on joue, qu'on peut toujours réussir avec sa propre bibliothèque, qui laissent une trace (points, badges, série), et des versus entre amis qui se jouent vraiment jusqu'au bout.

## Ce qui existe (relevé dans le code)

| Élément | Où | État |
|---|---|---|
| Pool de 18 défis, 2 tirés par semaine | `App.tsx` ~3927 (`CHALLENGE_POOL`, `getActiveWeeklyChallenges`) | marche, avec les défauts ci-dessous |
| Progression | `App.tsx` ~3655, à la fin de chaque session | seulement après « Rejoindre » |
| Table `challenge_participants` | `scratch/supabase_challenges_schema.sql` | progression écrite par le client |
| Page Défis | `App.tsx` ~8136 | 2 cartes + top 10 de tous les participants |
| Défi sur l'Accueil | `App.tsx` ~5610 | le premier des deux |
| Versus | `App.tsx` ~4101, table `versus_challenges` | création / accepter / refuser seulement |

Défauts à corriger par la refonte :

1. Inscription obligatoire : ce qui est joué avant « Rejoindre » ne compte pas.
2. 14 défis sur 18 visent une série (Zelda, GTA…) sans vérifier que le joueur en a un jeu.
3. Mots-clés trop courts (`ff`, `dk`, `link`) : faux positifs sur n'importe quel titre.
4. La semaine est comptée depuis le 1er janvier : rotation le jeudi en 2026, le vendredi en 2027, alors que la carte affiche lundi → dimanche.
5. Le tirage utilise `sort(() => rng() - 0.5)`, dont le résultat dépend du moteur JavaScript.
6. Aucune récompense gardée, aucun historique.
7. « Lance 5 jeux différents » compte des sessions, pas des jeux différents.
8. « j restants » et « participants » sont en dur en français ; la notification de réussite affiche le titre français quel que soit la langue.
9. Versus : `challenger_progress`, `opponent_progress` et `winner_id` ne sont jamais écrits. Un versus reste à 0–0 et ne se termine pas.
10. La progression est modifiable par chacun (politique RLS « Users update own progress »).

## Le principe : un journal de sessions, et des défis calculés dessus

Aujourd'hui `playtime.json` ne garde que des totaux (par jeu, par mois, et la liste des jours joués). Impossible d'en déduire « 2 h cette semaine sur N64 ». Tout le reste du plan repose donc sur une brique neuve :

- **Journal des sessions** (Rust, `sessions.rs`) : une ligne par session dans `%LOCALAPPDATA%/EmuWorld/sessions.json` — console, jeu, début, durée, émulateur. Écrit par `record_session`, gardé 400 jours comme `active_days`, écriture atomique.
- **Moteur de défis** (TypeScript pur, `src/challenges.ts`) : une fonction `progression(défi, sessions, période)`. Pas d'état caché, pas de compteur à part : on peut recalculer à tout moment, et un défi commencé en retard compte quand même tout ce qui a été joué dans la période.

Conséquences : plus d'inscription, plus de progression perdue, et la même fonction sert aux défis et aux versus.

## Les défis

### Rythmes

| Rythme | Nombre | Période | Exemple |
|---|---|---|---|
| Jour | 1 | minuit → minuit (heure locale) | « Joue 20 minutes à un jeu jamais lancé » |
| Semaine | 3 | lundi → dimanche (semaine ISO) | « 3 h sur Nintendo 64 » |
| Mois | 1 | mois civil, relié au Wrap | « Joue 15 jours ce mois-ci » |

### Communs et personnels

Un défi tiré de ta bibliothèque n'est pas le même que celui de ton ami : pas de classement possible. Donc deux familles :

- **Communs** (identiques pour tout le monde, classement entre amis) : 2 des 3 défis de la semaine et le défi du mois. Ils ne dépendent d'aucun jeu précis : temps total, jeux différents, jours joués, sessions en lobby.
- **Personnels** (tirés de ta bibliothèque, sans classement) : le défi du jour et 1 défi de la semaine. Le tirage ne garde que ce que tu peux réussir.

### Types de défi

| Type | Mesure | Source |
|---|---|---|
| `temps` | secondes jouées, avec filtre optionnel (console, série, jeu) | journal |
| `jeux_differents` | nombre de jeux distincts | journal |
| `jours` | nombre de jours avec au moins N minutes | journal |
| `sessions` | nombre de sessions d'au moins N minutes | journal |
| `decouverte` | temps sur un jeu jamais lancé avant la période | journal + playtime |
| `retour` | temps sur un favori pas joué depuis 30 jours | journal + playtime |
| `multi` | sessions jouées dans un lobby | journal (marqueur lobby) |
| `succes` | succès RetroAchievements débloqués | plus tard, étape 7 |

Les séries passent par une table de correspondance en mots entiers (`\bzelda\b`), et un défi de série n'est proposé que si le filtre trouve au moins un jeu dans la bibliothèque.

### Tirage

- Clé de période : `J-2026-11-12`, `S-2026-W46`, `M-2026-11`.
- Graine = clé de période (+ identifiant du joueur pour les personnels), mélange de Fisher-Yates : même résultat partout.
- Pas deux fois le même type dans la même semaine, pas le même défi deux semaines de suite.
- Les textes passent tous par `t("challengePool.…")` avec les valeurs en paramètres.

## Les récompenses

- **Points** : jour 10, semaine 30, mois 100. Bonus de 20 si les 3 défis de la semaine sont faits.
- **Série** : nombre de semaines d'affilée avec au moins un défi réussi, affichée sur l'Accueil.
- **Badges** : un badge par famille de défi, à paliers (bronze 1, argent 5, or 20 réussites), visibles sur le profil et le web panel.
- **Historique** : la liste des défis réussis, semaine par semaine.

À trancher — points et badges propres aux défis (par défaut), ou rangés dans les succès existants.

## Les versus

Même moteur, sur la période qui va de l'acceptation à l'échéance.

- Colonnes à ajouter : `started_at` (date d'acceptation), `rules` (le défi en JSON, à la place du texte libre), `finished_at`.
- Chaque joueur écrit **sa** progression à la fin de ses sessions et à l'ouverture de la page.
- À l'échéance, le versus passe en « terminé » et désigne le vainqueur. Pour qu'aucun des deux ne puisse se déclarer gagnant, cette clôture passe par une fonction Supabase.
- Dans l'interface : barre face à face, temps restant, notification à la fin, bouton « Revanche » qui recrée le même versus.
- Types : temps de jeu, jeux différents, jours joués. Le type `streak` actuel devient `jours`.

## Supabase

Nouveau fichier `scratch/supabase_challenges_v2.sql` (rien n'est exécuté sans ton accord) :

- `challenge_progress` — `user_id`, `challenge_key`, `period_key`, `progress`, `goal`, `completed_at`, `updated_at`, clé `(user_id, challenge_key, period_key)`. Lecture publique pour les classements, écriture par son propriétaire.
- `challenge_rewards` — points et badges gagnés, une ligne par réussite.
- `versus_challenges` — les trois colonnes ci-dessus, plus la fonction `close_versus(id)`.
- `challenge_participants` reste en place, en lecture seule, le temps que les anciennes versions disparaissent.

À trancher — anti-triche :

- **Par défaut : progression déclarée par l'app**, avec des garde-fous côté base (progression ≤ objectif, jamais en baisse, pas plus de 24 h de jeu par jour). Suffisant entre amis.
- **Option : progression calculée côté serveur** à partir des sessions envoyées. Plus solide, mais il faut envoyer le journal des sessions à Supabase et y écrire des fonctions.

## L'interface

- **Accueil** : la carte « Défi de la semaine » devient « Tes défis » — le défi du jour, les 3 de la semaine en barres, la série en cours.
- **Succès → onglet Défis** : défis en cours par rythme, points, badges, historique.
- **Social → onglet Défis** : classement des amis sur les défis communs, versus en cours et terminés.
- Fin de session : le récap affiche l'avancée des défis touchés par la partie.
- Chaque élément cliquable porte `gamepad-nav-item` ; composants du kit v3 (`.v3-card`, `.v3-progress`, `.v3-pill`).

À trancher — garder une page Défis unique dans Social (comme aujourd'hui) plutôt que deux onglets.

## Étapes

Chaque étape est livrable seule, avec ses tests, et laisse `npm run test:all` au vert.

1. **Journal des sessions** — `sessions.rs`, écrit par `record_session`, commande `get_sessions(depuis)`, handler dans `tauriMock.ts`. Tests Rust : ajout, plafond de 400 jours, fichier corrompu.
2. **Moteur** — `src/challenges.ts` : types, catalogue, périodes ISO, tirage, progression. Tests Vitest : chaque type, bornes de période (dimanche 23:59 / lundi 00:00), tirage stable, aucun défi impossible pour une bibliothèque donnée.
3. **Défis en local** — brancher le moteur à la place de `CHALLENGE_POOL` : plus de « Rejoindre », progression recalculée, textes traduits. À ce stade tout marche hors ligne et sans compte.
4. **Supabase** — `challenge_progress`, synchronisation à la fin des sessions, classement entre amis.
5. **Récompenses** — points, série, badges, historique ; affichage sur le profil et le web panel.
6. **Versus** — nouvelles colonnes, progression en direct, clôture, revanche.
7. **Plus tard** — défis RetroAchievements et speedrun, défi communautaire.

Les étapes 1 à 3 corrigent déjà les défauts 1 à 8 sans toucher à la base.

## À trancher

| Sujet | Par défaut |
|---|---|
| Anti-triche | progression déclarée par l'app, avec garde-fous |
| Récompenses | points et badges propres aux défis |
| Défi communautaire (objectif commun à tous) | non, étape 7 si l'envie vient |
| Place dans l'app | Accueil + onglet dans Succès + onglet dans Social |
| Défi du jour | oui, 1 par jour |

## Risques

- **Journal des sessions** : c'est une nouvelle donnée locale. Elle part de zéro à l'installation de la mise à jour ; les défis de la première semaine ne verront que les sessions jouées après.
- **Heure locale** : deux amis dans des fuseaux différents n'ont pas exactement la même semaine. Acceptable pour un classement entre amis.
- **Plusieurs PC** : le journal est local. Tant que la synchronisation multi-PC n'existe pas, la progression envoyée est celle du PC qui a joué.
- **Anciennes versions** : elles continueront d'écrire dans `challenge_participants`, d'où la table laissée en place.
