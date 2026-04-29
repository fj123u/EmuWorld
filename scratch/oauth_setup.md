# OAuth Setup (Google / Discord)

Guide pour faire marcher les 2 providers sociaux dans l'app desktop.
Le flow : app → ouvre le navigateur chez le provider → redirect vers Supabase →
Supabase redirect vers `emuworld://auth-callback` → Windows rappelle EmuWorld →
`oauth-callback` event → `exchangeCodeForSession` → utilisateur connecté.

## 1 · Supabase — redirect URL whitelist

Dashboard Supabase → **Authentication → URL Configuration** :

- **Site URL** : `emuworld://auth-callback` (si c'est pour le desktop)
- **Redirect URLs** (liste, ajouter chaque ligne) :
  - `emuworld://auth-callback`
  - `emuworld://*`
  - `https://emuworld.alwaysdata.net/**` *(pour le site web)*

Sans ça Supabase va rejeter le `redirectTo: 'emuworld://auth-callback'` qu'on
passe à `signInWithOAuth` et retomber sur sa URL par défaut.

## 2 · Supabase — activer les providers

Dashboard Supabase → **Authentication → Providers** :

### Google
1. Toggle **Google** on
2. Ouvre un nouvel onglet sur [console.cloud.google.com](https://console.cloud.google.com/apis/credentials)
3. Crée un projet "EmuWorld" (ou prends-en un existant)
4. **APIs & Services → Credentials → Create Credentials → OAuth client ID**
5. Type **Web application**
6. Name: `EmuWorld`
7. **Authorized redirect URIs** : colle EXACTEMENT
   ```
   https://yizxrntlerzfniqkdvfg.supabase.co/auth/v1/callback
   ```
   (c'est cette URL que Supabase t'affiche dans le panel Google Provider)
8. Save → copie le **Client ID** et **Client Secret** → retourne dans Supabase,
   colle-les dans les champs Google, Save.

### Discord
1. [discord.com/developers/applications](https://discord.com/developers/applications) → **New Application** (ou prends l'app EmuWorld que t'as déjà pour Rich Presence)
2. Sidebar **OAuth2 → General**
3. **Redirects** → Add Redirect :
   ```
   https://yizxrntlerzfniqkdvfg.supabase.co/auth/v1/callback
   ```
4. Save Changes
5. Copie le **Client ID** (page principale) et **Client Secret** (OAuth2 → reset secret si nécessaire)
6. Supabase → **Discord** provider → colle les deux → Save

## 3 · Vérifier le scheme `emuworld://` côté OS

Le binaire Tauri enregistre `emuworld://` via `register_all()` au boot. Pour
tester que Windows voit bien le handler :

```powershell
# Dans PowerShell, après avoir lancé l'app au moins une fois :
Get-ItemProperty "HKCU:\Software\Classes\emuworld"
```

Tu dois voir une entrée avec `shell\open\command` qui pointe sur le binaire
EmuWorld. Si tu ne la vois pas, l'app doit être lancée en **Administrateur**
une première fois pour l'enregistrement global, ou tu build en release et
tu installes le .msi (l'installeur enregistre le scheme au niveau système).

## 4 · Tester

1. Lance `npm run tauri dev`
2. Clique sur un bouton social → navigateur s'ouvre chez le provider
3. Autorise → le navigateur redirige vers Supabase puis vers `emuworld://auth-callback?code=xxx`
4. Windows demande "Ouvrir EmuWorld ?" → cliquer oui
5. L'app reçoit l'event `oauth-callback`, échange le code, te connecte ✨

## Bugs fréquents

- **Le navigateur affiche "redirect_uri_mismatch"** → l'URL `https://<project>.supabase.co/auth/v1/callback` n'est pas configurée chez le provider (retour à l'étape 2)
- **Le navigateur t'amène sur une page blanche Supabase puis rien** → `emuworld://auth-callback` n'est pas dans la whitelist des Redirect URLs Supabase (étape 1)
- **Windows ouvre le navigateur mais ne revient jamais à l'app** → le scheme n'est pas enregistré. Build le .msi avec `npm run tauri build` et installe-le au moins une fois
- **"No session data found in callback URL"** dans la console → l'URL ne contient ni `code` ni `access_token`. Probablement une erreur côté Supabase, check le message d'erreur dans l'URL (`?error=...`)
