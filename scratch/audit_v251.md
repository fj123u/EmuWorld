# Audit de securite — EmuWorld v2.5.1

**Date :** 17 juin 2026
**Version auditee :** v2.5.1 (post-corrections audit v2.5.0)

---

## Resume

14 problemes identifies : 3 Critiques, 5 Hauts, 4 Moyens, 2 Faibles.

---

## CRITIQUE

### C1 — Aucun hash SHA256 reel dans les SetupFiles

**Severite :** Critique
**Fichier :** `src-tauri/src/emulators.rs` (48 entrees)

Le champ `expected_sha256` existe dans la struct `SetupFile` mais **toutes les 48 entrees ont `None`**. Les firmwares, BIOS et cores telecharges ne sont jamais verifies. Un attaquant MITM ou compromettant un serveur upstream peut injecter un executable malveillant.

**Impact :** Compromission complete du systeme utilisateur.

**Correction :** Generer les SHA256 de chaque fichier et les ajouter dans le catalogue :
```rust
expected_sha256: Some("e66fa1dc5820d254611fdcdba0662372df297f78...".to_string()),
```

---

### C2 — Injection guides/reviews sans moderation

**Severite :** Critique
**Tables :** `game_guides`, `game_reviews`

Tout utilisateur authentifie peut INSERT des guides/reviews visibles par TOUS (`SELECT USING (true)`). Pas de queue de moderation, pas de rate limit, pas de filtre de contenu.

**Confirme par le pentest :** Juju a injecte de faux guides visibles par tous.

**Correction :**
```sql
ALTER TABLE game_guides ADD COLUMN approved boolean DEFAULT false;
DROP POLICY "Guides are public" ON game_guides;
CREATE POLICY "Approved guides public" ON game_guides FOR SELECT
  USING (approved = true OR auth.uid() = user_id);
```

---

### C3 — URLs 1fichier non validees depuis les collections

**Severite :** Critique (si les collections communautaires sont actives)
**Fichier :** `src/App.tsx` (batch download)

Les collections partagees par la communaute peuvent contenir des URLs arbitraires. Verifier que le download 1fichier valide strictement le domaine avant de telecharger.

**Correction :** Verifier cote Rust que l'URL est bien `https://1fichier.com/` ou `https://www.1fichier.com/` avant tout download.

---

## HAUTE

### H1 — Challenge progress falsifiable (versus_challenges)

**Severite :** Haute
**Table :** `challenge_participants`

Un utilisateur peut UPDATE sa propre `progress` sans validation serveur. Permet de tricher dans les defis.

**Correction :** Contrainte CHECK + validation via Edge Function.

---

### H2 — Guides/reviews lisibles sans auth (anon key)

**Severite :** Haute
**Tables :** `game_guides`, `game_reviews`

Les policies SELECT sont `USING (true)` — lisibles avec juste la cle anon publique, sans aucune authentification.

**Correction :**
```sql
DROP POLICY "Guides are public" ON game_guides;
CREATE POLICY "Auth read guides" ON game_guides FOR SELECT USING (auth.uid() IS NOT NULL);
```

---

### H3 — OAuth callback : tokens dans le fragment URL

**Severite :** Haute
**Fichier :** `web-panel/auth-callback.html`

Les tokens restent dans l'historique navigateur tant que le fragment n'est pas nettoye. Le fallback manuel expose aussi le token.

**Correction :** Ajouter `window.location.hash = ''` immediatement apres parsing.

---

### H4 — Playtime public scrappable via anon key

**Severite :** Haute
**Table :** `playtime_games`

Les utilisateurs avec `public_profile = true` ont leur bibliotheque + temps de jeu lisible par quiconque possede la cle anon.

**Correction :** Restreindre la lecture aux utilisateurs authentifies.

---

### H5 — CSP autorise 'unsafe-inline' dans script-src

**Severite :** Haute
**Fichier :** `src-tauri/tauri.conf.json`

`script-src 'self' 'unsafe-inline'` permet l'execution de scripts inline. Si un XSS est trouve, il n'y a pas de barriere.

**Correction :** Retirer `'unsafe-inline'` de script-src.

---

## MOYENNE

### M1 — Pas de rate limit sur les demandes d'ami

**Severite :** Moyenne

Un utilisateur peut spammer des milliers de demandes d'ami (harassment).

**Correction :** Trigger PostgreSQL limitant a 20 demandes/heure.

---

### M2 — Spam presence via Realtime

**Severite :** Moyenne

La table `presence` accepte des updates illimitees. Un attaquant peut flood le WebSocket de tous les clients connectes.

**Correction :** Throttle (1 update / 3 secondes minimum) via trigger.

---

### M3 — DPAPI sans entropie supplementaire

**Severite :** Moyenne
**Fichier :** `src-tauri/src/dpapi.rs`

DPAPI sans `pOptionalEntropy` : tout processus tournant sous le meme compte Windows peut dechiffrer les credentials.

**Correction :** Ajouter une entropie applicative ou migrer vers Windows Credential Manager.

---

### M4 — Log de longueur de cle dans le CI

**Severite :** Moyenne
**Fichier :** `.github/workflows/release.yml:53`

`echo "Signing key length: ${#KEY} chars"` divulgue une metadata utile a un attaquant.

**Correction :** Remplacer par `echo "Signing key present: OK"`

---

## FAIBLE

### L1 — CSP manque form-action et frame-ancestors

**Severite :** Faible

Ajouter `form-action 'none'; frame-ancestors 'none'` pour completude.

---

### L2 — Code mort Myrient

**Severite :** Faible

Le code valide encore `myrient.erista.me` alors que le site a ferme. Code mort a supprimer.

---

## Matrice de priorite

| # | Severite | Difficulte exploit | Priorite |
|---|----------|-------------------|----------|
| C1 | Critique | Moyenne (MITM) | P0 |
| C2 | Critique | Facile (signup) | P0 |
| C3 | Critique | Moyenne | P1 |
| H1 | Haute | Facile | P2 |
| H2 | Haute | Facile (curl) | P2 |
| H3 | Haute | Moyenne | P2 |
| H4 | Haute | Facile | P3 |
| H5 | Haute | Difficile | P3 |
| M1 | Moyenne | Facile | P4 |
| M2 | Moyenne | Facile | P4 |
| M3 | Moyenne | Moyenne | P5 |
| M4 | Moyenne | Info | P5 |

---

## Roadmap corrections

**Immediat (v2.5.2) :**
- C1 : Populer les SHA256 de tous les SetupFiles
- C2 : Colonne `approved` + moderation guides
- H2 : Restreindre SELECT guides/reviews aux authentifies
- H5 : Retirer `'unsafe-inline'` du CSP

**Court terme (v2.6.0) :**
- H1 : CHECK constraint sur challenge_participants
- H3 : Nettoyer le fragment OAuth + validation format token
- H4 : Restreindre playtime aux authentifies
- M1 : Rate limit demandes d'ami

**Moyen terme :**
- M2 : Throttle presence
- M3 : Entropie DPAPI ou migration Credential Manager
- M4 : Nettoyer log CI
