# Release guide (alpha + auto-updater)

Workflow actuel : tu push un tag `vX.Y.Z(-alpha.N)` → GitHub Actions compile
le `.exe` setup, le signe avec la clé privée Tauri, crée une Release en
mode **draft + prerelease**, et upload `EmuWorld-setup.exe`, sa signature
et le `latest.json`. L'app installée chez les utilisateurs voit le nouveau
`latest.json` au boot suivant et propose la mise à jour automatique.

## 1 · One-time setup (à faire une seule fois)

### 1.1 Ajouter la signing key aux secrets du repo

La clé privée locale est dans `~/.tauri/emuworld.key`. Contenu :

```bash
cat ~/.tauri/emuworld.key
```

Sur GitHub → **repo → Settings → Secrets and variables → Actions → New repository secret** :

| Secret name                          | Valeur                                         |
|--------------------------------------|------------------------------------------------|
| `TAURI_SIGNING_PRIVATE_KEY`          | Le **contenu** entier du fichier `.key`        |

Un seul secret suffit — la clé a été générée sans password, le workflow
force `TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""` directement. Si tu décides
plus tard de protéger la clé par un password, ajoute un secret
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` et remplace la valeur `""` dans
le workflow par `${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}`.

⚠️ **Ne commit JAMAIS** `emuworld.key` dans le repo. La publique `.pub` est
déjà dans `tauri.conf.json` et c'est elle qui compte.

### 1.2 Sauvegarder la clé privée hors du PC

Copie `~/.tauri/emuworld.key` sur un cloud ou un USB. Si tu la perds,
**aucune future release ne pourra plus être livrée en update** aux
installations déjà faites — elles n'accepteront pas une clé différente.

## 2 · Release flow (chaque version)

### 2.1 Bump la version

- `package.json` → champ `version` : `0.1.0-alpha.1` par exemple
- `src-tauri/tauri.conf.json` → champ `version` : même valeur
- `src-tauri/Cargo.toml` → `version = "0.1.0-alpha.1"` (Cargo n'aime pas
  les tags préfixés `v`, faut juste le SemVer)

### 2.2 Commit + tag + push

```bash
git add -A
git commit -m "chore(release): v0.1.0-alpha.1"
git tag v0.1.0-alpha.1
git push origin main --tags
```

### 2.3 Attendre le build

GitHub Actions se déclenche automatiquement sur le push du tag. Onglet
**Actions** du repo → suivre le job. Dure 15-25 min pour une première
build (cache vide), 5-8 min ensuite.

### 2.4 Publier la release

À la fin du job tu as une release **draft + prerelease** dans l'onglet
Releases. Tu peux :

1. Éditer les release notes
2. Vérifier que `latest.json`, `EmuWorld-x.y.z-setup.exe` et son `.sig`
   sont bien listés dans les assets
3. Passer de **Draft** à **Published**

Dès que c'est publié, les installations existantes voient la nouvelle
version au prochain boot et proposent la mise à jour.

## 3 · Tester localement avant de publier

```bash
# Build local (signé avec la clé locale)
npx @tauri-apps/cli build
```

Le résultat :
- `src-tauri/target/release/bundle/nsis/EmuWorld_<ver>_x64-setup.exe` — l'installeur
- `src-tauri/target/release/bundle/nsis/EmuWorld_<ver>_x64-setup.exe.sig` — signature
- `src-tauri/target/release/bundle/msi/` — version MSI alternative

Installe le `.exe` → lance l'app → vérifie qu'elle démarre et que le
check d'update en dev ne crash pas (il échoue silencieusement car
GitHub Releases n'existe pas encore).

## 4 · Déboguer un échec d'update

Dans l'app installée, ouvre DevTools (Ctrl+Shift+I si les devtools ne
sont pas désactivées en release) et regarde la console. Les logs
`[EmuWorld] update available` / `update check skipped` t'indiquent où
ça coince. Causes typiques :

- **404 sur `latest.json`** → la release est encore en draft, pas publiée
- **Signature invalide** → la pubkey dans `tauri.conf.json` a changé
  entre la version installée et la nouvelle → les installations
  existantes ne peuvent plus recevoir d'update. Cas critique, d'où le
  **never change the pubkey** en production
- **Version identique** → le tag et le champ version dans les trois
  fichiers doivent être alignés

## 5 · Partager avec tes potes

Une fois la première release publiée, envoie-leur le lien direct :
```
https://github.com/fj123u/EmuWorld/releases/latest
```

Ils download le `-setup.exe`, l'installent, et chaque nouvelle version
qu'ils relancent proposera l'update automatique.
