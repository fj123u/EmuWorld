const fr = {
  // Navigation
  nav: {
    console: "Console",
    roms: "Roms",
    installed: "Installé",
    store: "Store",
    controller: "Manette",
    leaderboard: "Classement",
    friends: "Amis",
    stats: "Stats",
    backup: "Backup",
    settings: "Réglages",
    changelogs: "Changelogs",
    navigation: "Navigation",
    consoles: "Consoles",
    manufacturers: "Constructeurs",
    library: "Bibliothèque",
    allConsoles: "Toutes les consoles",
  },

  // Header / Page titles
  header: {
    emulators: "Émulateurs",
    library: "Bibliothèque",
    installedEmulators: "Émulateurs installés",
    store: "Store",
    controller: "Manette",
    leaderboard: "Classement",
    friends: "Amis",
    stats: "Statistiques",
    backup: "Cloud Backup",
    settings: "Réglages",
    changelogs: "Changelogs",
  },

  // Subtitles
  subtitle: {
    emulators: "Installez des émulateurs pour jouer à vos ROMs",
    library: "Vos jeux installés",
    installed: "Émulateurs prêts à l'emploi",
    store: "Téléchargez des jeux",
    controllerConnected: "Manette connectée",
    controllerNone: "Aucune manette détectée",
    leaderboard: "Comparez vos stats avec la communauté",
    friends: "Gérez vos amis et chattez",
    stats: "Vos statistiques de jeu",
    backup: "Sauvegardez vos parties dans le cloud",
    settings: "Configurez votre expérience",
    changelogs: "Historique des mises à jour",
  },

  // Settings
  settings: {
    theme: "Thème",
    customAccent: "Accent personnalisé",
    reset: "Reset",
    directories: "Répertoires",
    romsFolder: "Dossier ROMs",
    emulatorsFolder: "Dossier Émulateurs",
    browse: "Parcourir",
    maintenance: "Maintenance",
    coverCache: "Cache des covers",
    coverCacheDesc: "Effacer les images de cover stockées localement. Utile si certaines covers sont incorrectes.",
    clearCache: "Vider le cache",
    importExport: "Import / Export",
    importExportDesc: "Exportez toute votre config EmuWorld (répertoires, playtime, favoris, notes) en JSON pour la restaurer sur un autre PC.",
    export: "Exporter",
    import: "Importer",
    boxartLogs: "Logs de récupération des covers",
    noLogs: "Aucun log. Essayez de rafraîchir la cover d'un jeu.",
    language: "Langue",
    languageDesc: "Choisissez la langue de l'interface.",
    retroachievements: "RetroAchievements",
    raDesc: "Connectez votre compte RetroAchievements pour voir vos trophées et les activer automatiquement dans vos émulateurs.",
    raUsername: "Nom d'utilisateur",
    raApiKey: "Clé API",
    raPassword: "Mot de passe RA",
    raUsernamePlaceholder: "Votre pseudo RA",
    raApiKeyPlaceholder: "API key (retroachievements.org/settings)",
    raPasswordPlaceholder: "Pour connecter les émulateurs automatiquement",
    raSaveApi: "Sauvegarder API",
    raLogin: "Login RA",
    raConnecting: "Connexion...",
    raTokenActive: "Token RA actif",
    raConfigureEmulators: "Configurer les émulateurs",
    raInstallCores: "Installer les cores",
    raDownloading: "Téléchargement...",
    raCompletedGames: "Jeux complétés (100%)",
    raLoad: "Charger",
    raLoading: "Chargement...",
    raLoadPrompt: "Cliquez \"Charger\" pour voir vos jeux 100%.",
    raEmuDesc: "Active les achievements dans RetroArch, DuckStation, PCSX2, Dolphin et PPSSPP. Les jeux NES/SNES/GB/GBA/N64/DS seront lancés via RetroArch avec RA activé.",
    profile: "Profil",
  },

  // Game card
  game: {
    retry: "Réessayer",
    play: "Jouer",
    delete: "Supprimer la ROM",
    deleteConfirm: "Supprimer {name} ?",
    notes: "Notes",
    favorites: "Favoris",
    addFavorite: "Ajouter aux favoris",
    removeFavorite: "Retirer des favoris",
    launches: "{count} lancement(s)",
    noGames: "Aucun jeu trouvé",
    searchPlaceholder: "Rechercher un jeu...",
  },

  // Store
  store: {
    singleGames: "Jeux à l'unité",
    fullPacks: "Packs complets",
    download: "Télécharger",
    downloading: "Téléchargement...",
    downloaded: "Téléchargé",
    searchPlaceholder: "Rechercher...",
    size: "Taille",
    noResults: "Aucun résultat",
    rgsHint: "Pour les jeux Switch, Wii U et les packs complets, passe sur l'onglet RetroGameSets.",
    openingFolder: "Ouverture du dossier dans le navigateur...",
    batchDownload: "Téléchargement auto",
    batchDesc: "Colle les liens 1fichier (un par ligne) pour télécharger automatiquement les fichiers dans ton dossier ROMs.",
    batchPlaceholder: "https://1fichier.com/?abc123\nhttps://1fichier.com/?def456\n...",
    batchConsole: "Console (ex: Wii U)",
    batchPwd: "Mot de passe (optionnel)",
    batchStart: "Lancer",
    batchComplete: "Tous les téléchargements sont terminés !",
    downloadQueue: "Téléchargements",
    waitingRetry: "Attente (limite 1fichier)... Nouvel essai dans 60s",
    saveAsCollection: "Sauvegarder en collection",
    createCollection: "Créer une collection",
    collectionDesc: "Crée une collection de liens téléchargeables. Tu pourras ensuite chercher par nom et télécharger individuellement.",
    collectionName: "Nom de la collection",
    collectionLinksPlaceholder: "Un lien par ligne :\nhttps://1fichier.com/?abc123 NomDuJeu.zip\nhttps://1fichier.com/?def456 AutreJeu.7z",
    collectionCreated: "Collection créée !",
    myCollections: "Mes collections",
    links: "liens",
    downloadAll: "Tout télécharger",
    searchCollection: "Rechercher dans la collection...",
  },

  // Friends
  friends: {
    myFriends: "Mes amis",
    pending: "En attente",
    search: "Rechercher",
    searchPlaceholder: "Rechercher un pseudo...",
    addFriend: "Ajouter",
    accept: "Accepter",
    decline: "Refuser",
    remove: "Supprimer",
    online: "En ligne",
    playing: "En train de jouer",
    offline: "Hors ligne",
    noFriends: "Aucun ami pour le moment",
    noPending: "Aucune demande en attente",
    chat: "Message",
    viewProfile: "Voir le profil",
    sendMessage: "Envoyer un message...",
  },

  // Stats
  stats: {
    totalPlaytime: "Temps total",
    totalLaunches: "Lancements",
    gamesPlayed: "Jeux joués",
    favoriteGames: "Favoris",
    topGames: "Top jeux",
    topConsoles: "Top consoles",
    streak: "Streak",
    streakDays: "{count} jour(s)",
    heatmap: "Activité",
    thisWeek: "Cette semaine",
    thisMonth: "Ce mois",
    allTime: "Tout temps",
  },

  // Backup
  backup: {
    localSaves: "Sauvegardes locales",
    cloudSaves: "Sauvegardes cloud",
    upload: "Uploader",
    restore: "Restaurer",
    lastBackup: "Dernier backup",
    noSaves: "Aucune sauvegarde trouvée",
    scanning: "Scan en cours...",
  },

  // Leaderboard
  leaderboard: {
    weeklyRanking: "Classement de la semaine",
    refresh: "Rafraîchir",
    loading: "Chargement...",
    rank: "Rang",
    player: "Joueur",
    playtime: "Temps de jeu",
    games: "Jeux",
  },

  // Controller
  controller: {
    connected: "Manette connectée",
    notDetected: "Aucune manette détectée",
    deadzone: "Zone morte",
    mappings: "Mappings",
    action: "Action",
    button: "Bouton",
    confirm: "Confirmer / Lancer",
    back: "Retour",
    details: "Détails",
    favorite: "Favori",
    prevPage: "Page précédente",
    nextPage: "Page suivante",
    search: "Recherche",
    settingsBtn: "Controller",
  },

  // Emulators
  emulator: {
    install: "Installer",
    installing: "Installation...",
    installed: "Installé",
    launch: "Lancer",
    openFolder: "Ouvrir le dossier",
    website: "Site web",
    supportedFormats: "Formats supportés",
    category: "Catégorie",
  },
  emulators: {
    downloading: "Téléchargement de l'émulateur...",
    installChoice: "Choix de l'émulateur",
    installChoiceDesc: "Cet émulateur est aussi disponible via RetroArch. RetroArch permet le multijoueur en ligne (netplay). L'émulateur standalone est plus simple à configurer.",
    standalone: "Standalone",
    multiSupported: "Multijoueur supporté",
  },

  // Onboarding
  onboarding: {
    welcome: "Bienvenue sur EmuWorld !",
    welcomeDesc: "Configurons votre environnement en quelques étapes.",
    step1Title: "Dossier ROMs",
    step1Desc: "Où sont stockés vos jeux ?",
    step2Title: "Dossier Émulateurs",
    step2Desc: "Où installer les émulateurs ?",
    step3Title: "Compte",
    step3Desc: "Connectez-vous pour synchroniser vos données.",
    step4Title: "C'est parti !",
    step4Desc: "Tout est prêt. Amusez-vous bien !",
    next: "Suivant",
    skip: "Passer",
    finish: "Terminer",
    selectFolder: "Choisir un dossier",
  },

  // Tour
  tour: {
    storeTitle: "Le Store",
    storeDesc: "Téléchargez des jeux directement depuis ici.",
    emulatorsTitle: "Les Émulateurs",
    emulatorsDesc: "Installez les émulateurs dont vous avez besoin.",
    libraryTitle: "Votre Bibliothèque",
    libraryDesc: "Tous vos jeux installés apparaissent ici.",
    playTitle: "Jouer",
    playDesc: "Cliquez sur un jeu pour le lancer.",
    friendsTitle: "Amis",
    friendsDesc: "Ajoutez des amis et chattez ensemble.",
    next: "Suivant",
    prev: "Précédent",
    finish: "Terminer",
    stepOf: "{current} / {total}",
  },

  // Common / Misc
  common: {
    loading: "Chargement...",
    error: "Erreur",
    success: "Succès",
    cancel: "Annuler",
    save: "Sauvegarder",
    close: "Fermer",
    back: "Retour",
    confirm: "Confirmer",
    yes: "Oui",
    no: "Non",
    or: "ou",
    and: "et",
    noResults: "Aucun résultat",
    searchPlaceholder: "Rechercher...",
  },

  // Auth
  auth: {
    signIn: "Connexion",
    signUp: "Inscription",
    signOut: "Déconnexion",
    email: "Email",
    password: "Mot de passe",
    username: "Nom d'utilisateur",
    forgotPassword: "Mot de passe oublié ?",
    continueWith: "Continuer avec",
    orEmail: "ou par email",
    welcome: "Bienvenue !",
    accountCreated: "Compte créé ! Vérifie ton email pour confirmer.",
    connectedSuccess: "Connecté avec succès !",
    signedOut: "Déconnecté",
    emailNotConfirmed: "Confirme ton email avant de te connecter.",
    checkEmailTitle: "Vérifie ton email",
    checkEmailDesc: "Un email de confirmation vient d'être envoyé. Clique sur le lien dans le mail pour activer ton compte, puis reviens te connecter ici.",
  },

  // Gamepad labels
  gamepad: {
    confirm: "Confirmer",
    back: "Retour",
  },

  // Big Picture
  bigPicture: {
    exit: "Quitter",
    title: "Mode Big Picture",
    games: "jeux",
    game: "jeu",
  },

  // Toasts
  toast: {
    connected: "Connecté avec succès ! 🎉",
    accountCreated: "Compte créé ! Vérifiez votre email pour confirmer.",
    welcomeBack: "Bon retour ! 🎮",
    usernameUpdated: "Pseudo mis à jour ! ✨",
    avatarUpdated: "Avatar mis à jour ! 📸",
    signedOut: "Déconnecté avec succès",
    raSaved: "Identifiants RetroAchievements sauvegardés !",
    raConnected: "Connecté à RetroAchievements ! Token obtenu.",
    raConfigured: "RetroAchievements configuré dans : {emulators}",
    coresInstalled: "Cores installés : {cores}",
    configExported: "Configuration exportée !",
    configImported: "Configuration importée !",
    cacheCleaned: "Cache des covers vidé !",
    friendRequestSent: "Demande d'ami envoyée !",
    friendRequestAccepted: "Demande acceptée !",
    friendRemoved: "Ami supprimé",
  },

  // Lobby
  lobby: {
    title: "Lobby",
    created: "Lobby créé ! Invite un ami.",
    inviteSent: "Invitation envoyée !",
    inviteReceived: "Tu as reçu une invitation à un lobby !",
    dissolved: "Lobby dissous",
    left: "Tu as quitté le lobby",
    allReady: "Tous les joueurs sont prêts ! Lancement du netplay...",
    noRom: "Tu n'as pas \"{game}\" dans ta bibliothèque. Télécharge-le d'abord !",
    downloadGame: "Télécharger le jeu",
    ready: "Je suis prêt !",
    waiting: "En attente...",
    dissolve: "Dissoudre",
    leave: "Quitter",
    inviteFriend: "Inviter un ami...",
    createLobby: "Créer un lobby",
    you: "Toi",
  },

  // Versus
  versus: {
    title: "Défis en cours",
    challengeSent: "Défi envoyé à {name} !",
    accepted: "Défi accepté ! C'est parti !",
    declined: "Défi refusé",
    challenge: "Défier {name}",
    type: "Type de défi",
    playtime: "Temps de jeu",
    launches: "Nombre de lancements",
    streak: "Plus long streak",
    specificGame: "Jeu spécifique (optionnel)",
    allGames: "Tous les jeux...",
    duration: "Durée",
    days3: "3 jours",
    days7: "7 jours",
    days14: "14 jours",
    days30: "30 jours",
    cancel: "Annuler",
    sendChallenge: "Envoyer le défi",
    accept: "Accepter",
    decline: "Refuser",
    vs: "VS",
    endsAt: "Fin : {date}",
    whoPlaysMore: "Qui joue le plus en {days} jours ?",
    whoLaunchesMore: "Qui lance le plus de jeux en {days} jours ?",
    whoStreaks: "Qui maintient le plus long streak ?",
  },

  // Marketplace
  marketplace: {
    title: "Marketplace",
    publish: "Partager mon thème",
    published: "Thème publié !",
    applied: "Thème \"{name}\" appliqué !",
    themeName: "Nom du thème",
    themeDesc: "Description (optionnel)",
    downloads: "téléchargements",
    by: "par",
  },

  // Health check
  health: {
    title: "Intégrité des ROMs",
    description: "Vérifie que toutes tes ROMs sont valides (pas vides, pas corrompues).",
    check: "Vérifier l'intégrité",
    scanning: "Scan en cours...",
    allOk: "Toutes les ROMs sont OK !",
    issuesFound: "{count} problème(s) trouvé(s)",
    confirmDelete: "Supprimer les fichiers corrompus ?",
    deleted: "{count} fichier(s) supprimé(s)",
    empty: "Fichier vide (0 octets)",
    suspect: "Fichier suspect ({size} octets)",
    corrupt: "Archive corrompue",
    notFound: "Fichier introuvable",
  },

  // Guides
  guides: {
    title: "Guide",
    presentation: "Présentation",
    tips: "Astuces",
    achievements: "Succès",
    secrets: "Secrets",
    writeGuide: "Écrire un guide",
    publish: "Publier",
    cancel: "Annuler",
    titlePlaceholder: "Titre du guide...",
    contentPlaceholder: "Écris ton guide ici... (astuces, stratégies, walkthroughs...)",
    noGuides: "Aucun guide pour ce jeu. Sois le premier !",
  },

  // Reviews
  reviews: {
    title: "Avis communauté",
    writeReview: "Ton avis",
    submit: "Publier",
    noReviews: "Aucun avis pour ce jeu.",
  },

  // Tutorial
  tutorial: {
    title: "Tutoriel",
    restart: "Relancer le tutoriel de démarrage",
  },

  // Logs
  logs: {
    title: "Fichiers de logs",
    description: "Les logs sont sauvegardés dans des fichiers quotidiens (auto-suppression après 7 jours).",
    openFolder: "Ouvrir le dossier",
    openCurrent: "Ouvrir le log du jour",
  },

  // Updates
  updates: {
    checkButton: "Vérifier les mises à jour",
    checking: "Vérification des mises à jour...",
    allUpToDate: "Tous les émulateurs sont à jour !",
    available: "{count} mise(s) à jour disponible(s)",
    update: "Mettre à jour",
  },

  // Challenges
  challenges: {
    title: "Challenges",
    active: "challenges actifs",
    leaderboard: "Classement",
    progress: "Progression",
    joined: "Tu participes au challenge !",
    noneActive: "Aucun challenge actif cette semaine.",
  },

  // Library
  library: {
    sortPlaytime: "Temps joué",
    sortRating: "Note",
    sortLastPlayed: "Dernier joué",
    sortLaunches: "Nb launches",
    filterAll: "Tous",
    filterFavorites: "Favoris",
    filterUnplayed: "Non joués",
    filterRated: "Notés",
    collections: "Collections",
    noGames: "Aucun jeu dans la bibliothèque",
    noGamesDesc: "Ajoute des ROMs depuis le Store ou scanne un dossier pour commencer !",
    noScreenshots: "Pas de screenshots. Appuie sur Ctrl+F12 en jeu pour capturer.",
  },

  // Wrap
  wrap: {
    noData: "Pas encore de données ce mois-ci. Joue un peu et reviens !",
  },

  // Marketplace extra
  marketplaceEmpty: {
    noThemes: "Aucun thème partagé pour le moment.",
    beFirst: "Sois le premier à publier ton thème !",
  },

  // Session
  session: {
    ended: "Session terminée",
  },

  // RetroAchievements
  ra: {
    searching: "Recherche sur RetroAchievements...",
    notFound: "Impossible de trouver ce jeu sur RetroAchievements.",
  },

  // Guides extra
  guidesExtra: {
    noInSection: "Aucun guide dans cette section.",
  },
};

export type Translations = typeof fr;
export default fr;
