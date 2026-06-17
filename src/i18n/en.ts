import type { Translations } from "./fr";

const en: Translations = {
  // Navigation
  nav: {
    console: "Console",
    roms: "Roms",
    installed: "Installed",
    store: "Store",
    controller: "Controller",
    leaderboard: "Leaderboard",
    friends: "Friends",
    stats: "Stats",
    backup: "Backup",
    settings: "Settings",
    changelogs: "Changelogs",
    navigation: "Navigation",
    consoles: "Consoles",
    manufacturers: "Manufacturers",
    library: "Library",
    allConsoles: "All consoles",
  },

  // Header / Page titles
  header: {
    emulators: "Emulators",
    library: "Library",
    installedEmulators: "Installed Emulators",
    store: "Store",
    controller: "Controller",
    leaderboard: "Leaderboard",
    friends: "Friends",
    stats: "Statistics",
    backup: "Cloud Backup",
    settings: "Settings",
    changelogs: "Changelogs",
  },

  // Subtitles
  subtitle: {
    emulators: "Install emulators to play your ROMs",
    library: "Your installed games",
    installed: "Emulators ready to use",
    store: "Download games",
    controllerConnected: "Controller connected",
    controllerNone: "No controller detected",
    leaderboard: "Compare your stats with the community",
    friends: "Manage your friends and chat",
    stats: "Your gaming statistics",
    backup: "Save your game progress to the cloud",
    settings: "Configure your experience",
    changelogs: "Update history",
  },

  // Settings
  settings: {
    theme: "Theme",
    customAccent: "Custom accent",
    reset: "Reset",
    directories: "Directories",
    romsFolder: "ROMs Folder",
    emulatorsFolder: "Emulators Folder",
    browse: "Browse",
    maintenance: "Maintenance",
    coverCache: "Cover Art Cache",
    coverCacheDesc: "Clear locally stored boxart images. Useful if some covers are wrong.",
    clearCache: "Clear Cache",
    importExport: "Import / Export",
    importExportDesc: "Export your entire EmuWorld config (directories, playtime, favorites, notes) as JSON to restore on another PC.",
    export: "Export",
    import: "Import",
    boxartLogs: "Boxart Fetch Logs",
    noLogs: "No logs yet. Try refreshing a game's boxart.",
    language: "Language",
    languageDesc: "Choose the interface language.",
    retroachievements: "RetroAchievements",
    raDesc: "Connect your RetroAchievements account to see your trophies and auto-enable them in emulators.",
    raUsername: "Username",
    raApiKey: "API Key",
    raPassword: "RA Password",
    raUsernamePlaceholder: "Your RA username",
    raApiKeyPlaceholder: "API key (retroachievements.org/settings)",
    raPasswordPlaceholder: "To auto-connect emulators",
    raSaveApi: "Save API",
    raLogin: "Login RA",
    raConnecting: "Connecting...",
    raTokenActive: "RA Token active",
    raConfigureEmulators: "Configure emulators",
    raInstallCores: "Install cores",
    raDownloading: "Downloading...",
    raCompletedGames: "Completed Games (100%)",
    raLoad: "Load",
    raLoading: "Loading...",
    raLoadPrompt: "Click \"Load\" to see your 100% games.",
    raEmuDesc: "Enables achievements in RetroArch, DuckStation, PCSX2, Dolphin, and PPSSPP. NES/SNES/GB/GBA/N64/DS games will launch via RetroArch with RA enabled.",
    profile: "Profile",
  },

  // Game card
  game: {
    retry: "Retry",
    play: "Play",
    delete: "Delete ROM",
    deleteConfirm: "Delete {name}?",
    notes: "Notes",
    favorites: "Favorites",
    addFavorite: "Add to favorites",
    removeFavorite: "Remove from favorites",
    launches: "{count} launch(es)",
    noGames: "No games found",
    searchPlaceholder: "Search a game...",
  },

  // Store
  store: {
    singleGames: "Single games",
    fullPacks: "Full packs",
    download: "Download",
    downloading: "Downloading...",
    downloaded: "Downloaded",
    searchPlaceholder: "Search...",
    size: "Size",
    noResults: "No results",
    rgsHint: "For Switch, Wii U games and full packs, switch to the RetroGameSets tab.",
    openingFolder: "Opening folder in browser...",
    batchDownload: "Auto download",
    batchDesc: "Paste 1fichier links (one per line) to automatically download files to your ROMs folder.",
    batchPlaceholder: "https://1fichier.com/?abc123\nhttps://1fichier.com/?def456\n...",
    batchConsole: "Console (e.g. Wii U)",
    batchPwd: "Password (optional)",
    batchStart: "Start",
    batchComplete: "All downloads complete!",
    downloadQueue: "Downloads",
    waitingRetry: "Waiting (1fichier rate limit)... Retrying in 60s",
    openedInBrowser: "Opened in browser (1fichier rate limit)",
    cancelled: "Cancelled",
    saveAsCollection: "Save as collection",
    createCollection: "Create collection",
    collectionDesc: "Create a collection of downloadable links. You can then search by name and download individually.",
    collectionName: "Collection name",
    collectionLinksPlaceholder: "One link per line:\nhttps://1fichier.com/?abc123 GameName.zip\nhttps://1fichier.com/?def456 OtherGame.7z",
    collectionCreated: "Collection created!",
    myCollections: "My collections",
    links: "links",
    downloadAll: "Download all",
    searchCollection: "Search in collection...",
    resolveNames: "Fetch names",
    resolving: "Resolving...",
    share: "Share",
    collectionShared: "Collection shared with the community!",
    extracting: "Extracting...",
    done: "Done!",
  },

  // Friends
  friends: {
    myFriends: "My friends",
    pending: "Pending",
    search: "Search",
    searchPlaceholder: "Search a username...",
    addFriend: "Add",
    accept: "Accept",
    decline: "Decline",
    remove: "Remove",
    online: "Online",
    playing: "Playing",
    offline: "Offline",
    noFriends: "No friends yet",
    noPending: "No pending requests",
    chat: "Message",
    viewProfile: "View profile",
    sendMessage: "Send a message...",
  },

  // Stats
  stats: {
    totalPlaytime: "Total playtime",
    totalLaunches: "Launches",
    gamesPlayed: "Games played",
    favoriteGames: "Favorites",
    topGames: "Top games",
    topConsoles: "Top consoles",
    streak: "Streak",
    streakDays: "{count} day(s)",
    heatmap: "Activity",
    thisWeek: "This week",
    thisMonth: "This month",
    allTime: "All time",
  },

  // Backup
  backup: {
    localSaves: "Local saves",
    cloudSaves: "Cloud saves",
    upload: "Upload",
    restore: "Restore",
    lastBackup: "Last backup",
    noSaves: "No saves found",
    scanning: "Scanning...",
  },

  // Leaderboard
  leaderboard: {
    weeklyRanking: "Weekly ranking",
    refresh: "Refresh",
    loading: "Loading...",
    rank: "Rank",
    player: "Player",
    playtime: "Playtime",
    games: "Games",
  },

  // Controller
  controller: {
    connected: "Controller connected",
    notDetected: "No controller detected",
    deadzone: "Deadzone",
    mappings: "Mappings",
    action: "Action",
    button: "Button",
    confirm: "Confirm / Launch",
    back: "Back",
    details: "Details",
    favorite: "Favorite",
    prevPage: "Previous page",
    nextPage: "Next page",
    search: "Search",
    settingsBtn: "Controller",
  },

  // Emulators
  emulator: {
    install: "Install",
    installing: "Installing...",
    installed: "Installed",
    launch: "Launch",
    openFolder: "Open folder",
    website: "Website",
    supportedFormats: "Supported formats",
    category: "Category",
  },
  emulators: {
    downloading: "Downloading emulator...",
    installChoice: "Choose emulator",
    installChoiceDesc: "This emulator is also available via RetroArch. RetroArch enables online multiplayer (netplay). The standalone emulator is easier to configure.",
    standalone: "Standalone",
    multiSupported: "Multiplayer supported",
  },

  // Onboarding
  onboarding: {
    welcome: "Welcome to EmuWorld!",
    welcomeDesc: "Let's set up your environment in a few steps.",
    step1Title: "ROMs Folder",
    step1Desc: "Where are your games stored?",
    step2Title: "Emulators Folder",
    step2Desc: "Where to install emulators?",
    step3Title: "Account",
    step3Desc: "Sign in to sync your data.",
    step4Title: "Let's go!",
    step4Desc: "Everything is ready. Have fun!",
    next: "Next",
    skip: "Skip",
    finish: "Finish",
    selectFolder: "Choose a folder",
  },

  // Tour
  tour: {
    storeTitle: "The Store",
    storeDesc: "Download games directly from here.",
    emulatorsTitle: "Emulators",
    emulatorsDesc: "Install the emulators you need.",
    libraryTitle: "Your Library",
    libraryDesc: "All your installed games appear here.",
    playTitle: "Play",
    playDesc: "Click a game to launch it.",
    friendsTitle: "Friends",
    friendsDesc: "Add friends and chat together.",
    next: "Next",
    prev: "Previous",
    finish: "Finish",
    stepOf: "{current} / {total}",
  },

  // Common / Misc
  common: {
    loading: "Loading...",
    error: "Error",
    success: "Success",
    cancel: "Cancel",
    save: "Save",
    close: "Close",
    back: "Back",
    confirm: "Confirm",
    yes: "Yes",
    no: "No",
    or: "or",
    and: "and",
    noResults: "No results",
    searchPlaceholder: "Search...",
  },

  // Auth
  auth: {
    signIn: "Sign In",
    signUp: "Sign Up",
    signOut: "Sign Out",
    email: "Email",
    password: "Password",
    username: "Username",
    forgotPassword: "Forgot password?",
    continueWith: "Continue with",
    orEmail: "or by email",
    welcome: "Welcome!",
    accountCreated: "Account created! Check your email to confirm.",
    connectedSuccess: "Connected successfully!",
    signedOut: "Signed out",
    emailNotConfirmed: "Please confirm your email before signing in.",
    checkEmailTitle: "Check your email",
    checkEmailDesc: "A confirmation email has been sent. Click the link in the email to activate your account, then come back and sign in.",
    invalidCredentials: "Incorrect email or password. Don't have an account yet?",
    deleteAccount: "Delete my account",
    deleteAccountConfirm: "Are you sure you want to delete your account? All your data will be permanently erased.",
    deleteAccountSuccess: "Account deleted successfully.",
    deleteAccountError: "Error deleting account.",
  },

  // Gamepad labels
  gamepad: {
    confirm: "Confirm",
    back: "Back",
  },

  // Big Picture
  bigPicture: {
    exit: "Exit",
    title: "Big Picture Mode",
    games: "games",
    game: "game",
  },

  // Toasts
  toast: {
    connected: "Connected successfully! 🎉",
    accountCreated: "Account created! Check your email to confirm.",
    welcomeBack: "Welcome back! 🎮",
    usernameUpdated: "Username updated! ✨",
    avatarUpdated: "Avatar updated! 📸",
    signedOut: "Signed out successfully",
    raSaved: "RetroAchievements credentials saved!",
    raConnected: "Connected to RetroAchievements! Token obtained.",
    raConfigured: "RetroAchievements configured in: {emulators}",
    coresInstalled: "Cores installed: {cores}",
    configExported: "Configuration exported!",
    configImported: "Configuration imported!",
    cacheCleaned: "Cover cache cleared!",
    friendRequestSent: "Friend request sent!",
    friendRequestAccepted: "Request accepted!",
    friendRemoved: "Friend removed",
  },

  // Lobby
  lobby: {
    title: "Lobby",
    created: "Lobby created! Invite a friend.",
    inviteSent: "Invitation sent!",
    inviteReceived: "You received a lobby invitation!",
    dissolved: "Lobby dissolved",
    left: "You left the lobby",
    allReady: "All players are ready! Launching netplay...",
    noRom: "You don't have \"{game}\" in your library. Download it first!",
    downloadGame: "Download game",
    ready: "I'm ready!",
    waiting: "Waiting...",
    dissolve: "Dissolve",
    leave: "Leave",
    inviteFriend: "Invite a friend...",
    createLobby: "Create a lobby",
    launch: "Launch game",
    you: "You",
  },

  // Versus
  versus: {
    title: "Active challenges",
    challengeSent: "Challenge sent to {name}!",
    accepted: "Challenge accepted! Let's go!",
    declined: "Challenge declined",
    challenge: "Challenge {name}",
    type: "Challenge type",
    playtime: "Playtime",
    launches: "Number of launches",
    streak: "Longest streak",
    specificGame: "Specific game (optional)",
    allGames: "All games...",
    duration: "Duration",
    days3: "3 days",
    days7: "7 days",
    days14: "14 days",
    days30: "30 days",
    cancel: "Cancel",
    sendChallenge: "Send challenge",
    accept: "Accept",
    decline: "Decline",
    vs: "VS",
    endsAt: "Ends: {date}",
    whoPlaysMore: "Who plays the most in {days} days?",
    whoLaunchesMore: "Who launches the most games in {days} days?",
    whoStreaks: "Who maintains the longest streak?",
  },

  // Marketplace
  marketplace: {
    title: "Marketplace",
    publish: "Share my theme",
    published: "Theme published!",
    applied: "Theme \"{name}\" applied!",
    themeName: "Theme name",
    themeDesc: "Description (optional)",
    downloads: "downloads",
    by: "by",
  },

  // Health check
  health: {
    title: "ROM Integrity",
    description: "Check that all your ROMs are valid (not empty, not corrupted).",
    check: "Check integrity",
    scanning: "Scanning...",
    allOk: "All ROMs are OK!",
    issuesFound: "{count} issue(s) found",
    confirmDelete: "Delete corrupted files?",
    deleted: "{count} file(s) deleted",
    empty: "Empty file (0 bytes)",
    suspect: "Suspicious file ({size} bytes)",
    corrupt: "Corrupted archive",
    notFound: "File not found",
  },

  // Guides
  guides: {
    title: "Guide",
    presentation: "Overview",
    tips: "Tips",
    achievements: "Achievements",
    secrets: "Secrets",
    writeGuide: "Write a guide",
    publish: "Publish",
    cancel: "Cancel",
    titlePlaceholder: "Guide title...",
    contentPlaceholder: "Write your guide here... (tips, strategies, walkthroughs...)",
    noGuides: "No guides for this game yet. Be the first!",
  },

  // Reviews
  reviews: {
    title: "Community reviews",
    writeReview: "Your review",
    submit: "Submit",
    noReviews: "No reviews for this game.",
  },

  // Tutorial
  tutorial: {
    title: "Tutorial",
    restart: "Restart the getting started tutorial",
  },

  // Logs
  logs: {
    title: "Log files",
    description: "Logs are saved to daily files (auto-deleted after 7 days).",
    openFolder: "Open folder",
    openCurrent: "Open today's log",
  },

  // Updates
  updates: {
    checkButton: "Check for updates",
    checking: "Checking for updates...",
    allUpToDate: "All emulators are up to date!",
    available: "{count} update(s) available",
    update: "Update",
  },

  // Challenges
  challenges: {
    title: "Challenges",
    active: "active challenges",
    leaderboard: "Leaderboard",
    progress: "Progress",
    joined: "You joined the challenge!",
    noneActive: "No active challenges this week.",
  },

  // Library
  library: {
    sortPlaytime: "Playtime",
    sortRating: "Rating",
    sortLastPlayed: "Last played",
    sortLaunches: "Launches",
    filterAll: "All",
    filterFavorites: "Favorites",
    filterUnplayed: "Unplayed",
    filterRated: "Rated",
    collections: "Collections",
    noGames: "No games in your library",
    noGamesDesc: "Add ROMs from the Store or scan a folder to get started!",
    noScreenshots: "No screenshots yet. Press Ctrl+F12 while playing to capture.",
  },

  // Wrap
  wrap: {
    noData: "No data yet this month. Play some games and come back!",
  },

  // Marketplace extra
  marketplaceEmpty: {
    noThemes: "No shared themes yet.",
    beFirst: "Be the first to publish your theme!",
  },

  // Session
  session: {
    ended: "Session ended",
  },

  // RetroAchievements
  ra: {
    searching: "Searching on RetroAchievements...",
    notFound: "Could not find this game on RetroAchievements.",
  },

  // Guides extra
  guidesExtra: {
    noInSection: "No guides in this section.",
  },
};

export default en;
