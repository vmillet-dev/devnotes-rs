import { Lang } from '../../core/lang';

/**
 * What the scenes show, in each language: the application's own wording (crates/i18n), and the
 * notes of the capture above them (`app::screens`' seed), so that both pages tell one story.
 */
const FR = {
  copied: 'Copié',
  list: {
    title: 'Avant la démonstration',
    priority: 'Haute',
    items: ['Vider le cache du navigateur', 'Préparer le jeu de données', 'Couper les notifications'],
    space: 'Projets',
  },
  snippet: {
    query: 'relancer',
    group: 'NOTES',
    title: 'Relancer le service de paiement',
    fields: '2 champs à remplir',
    keys: ['naviguer', 'ouvrir', 'copier', 'fermer'],
    form: 'Remplir les champs',
    hint: 'Ce snippet attend 2 valeurs.',
    cancel: 'Annuler',
    copy: 'Copier rempli',
    service: 'paiement-api',
    namespace: 'recette',
  },
  drawing: {
    title: 'Schéma du réseau',
    browser: 'Navigateur',
    balancer: ['Répartiteur', 'de charge'],
    api: ['API paiement 1', 'API paiement 2'],
    note: 'HTTPS terminé ici',
    exports: ['Exporter en PNG', 'Exporter en SVG'],
  },
  request: {
    title: 'Lister les factures',
    env: 'recette',
    host: 'https://api.recette.interne',
    path: '/factures?statut=impayee',
    links: ['requête', 'dossier', 'collection'],
    found: 'environnement « recette »',
    send: 'Envoyer',
    sending: 'Envoi…',
    size: '2 o',
  },
  jwt: {
    tools: 'Outils ›',
    input: 'ENTRÉE',
    header: 'EN-TÊTE',
    payload: 'CHARGE UTILE',
    load: 'eyJzdWIiOiJmYWN0dXJhdGlvbiIsInNjb3BlIjoiZmFjdHVyZXM6bGVjdHVyZSIsImlhdCI6MTc5NTY5MTg4MCwiZXhwIjoxNzk1Njk1NDgwfQ',
    claims: ['"sub": "facturation",', '"scope": "factures:lecture"'],
    issued: 'Émis · iat',
    expires: 'Expiration · exp',
    state: 'Expiré · il y a 12 minutes',
  },
  diff: {
    plus: '+ 4 lignes',
    minus: '− 3 lignes',
    was: 'A  nginx recette',
    now: 'B  nginx production',
  },
  search: {
    query: 'impayee',
    count: '1 résultat',
    rows: [
      { title: 'Relancer le service de paiement', where: 'Exploitation' },
      { title: 'Factures impayées de mars', where: 'Projets' },
      { title: 'Avant la démonstration', where: 'Projets' },
      { title: 'Budget du déplacement', where: 'Découverte' },
      { title: 'Procédure de mise en production', where: 'Livraisons' },
    ],
    /** The found title around its match, and the line quoted. */
    match: ['Factures ', 'impayées', ' de mars'],
    cite: ["WHERE statut = '", 'impayee', "'"],
  },
  calc: {
    title: 'Budget du déplacement',
    train: 'train = 2 × 89 €',
    hotel: ['hôtel = 3 × ', '112', '104', ' €'],
    meals: 'repas = 6 × 18 €',
    total: 'total = train + hôtel + repas',
    results: ['178,00 €', '336,00 €', '312,00 €', '108,00 €', '622,00 €', '598,00 €'],
  },
  note: {
    title: 'Procédure de mise en production',
    tag: '#livraison',
    heading: 'Mise en production',
    lead: ['Trois étapes, ', 'dans cet ordre', ' :'],
    first: ['Geler la branche ', 'dev', ''],
    steps: ['Lancer la chaîne de publication', 'Prévenir l’astreinte'],
    head: ['Étape', 'Durée'],
    rows: [
      ['Build', '12 min'],
      ['Déploiement', '4 min'],
    ],
    saved: 'Enregistrée en Markdown',
    markdown:
      '# Mise en production\nTrois étapes, **dans cet ordre** :\n1. Geler la branche `dev`\n2. Lancer la chaîne de publication\n3. Prévenir l’astreinte\n\n| Étape       | Durée  |\n| ----------- | ------ |\n| Build       | 12 min |\n| Déploiement | 4 min  |',
    present: 'Présenter',
  },
  lock: {
    library: 'Bibliothèque « Travail »',
    state: 'Chiffrée sur le disque',
    rows: [
      ['Relancer le service de paiement', '#exploitation', 'il y a 1j'],
      ['Factures impayées de mars', '#sql', 'il y a 2j'],
      ['Budget du déplacement', '#voyage', '16:20'],
      ['Procédure de mise en production', '#livraison', '17:30'],
    ],
  },
};

export type SceneWords = typeof FR;

const EN: SceneWords = {
  copied: 'Copied',
  list: {
    title: 'Before the demo',
    priority: 'High',
    items: ['Clear the browser cache', 'Prepare the data set', 'Turn off notifications'],
    space: 'Projects',
  },
  snippet: {
    query: 'restart',
    group: 'NOTES',
    title: 'Restart the payment service',
    fields: '2 fields to fill',
    keys: ['move', 'open', 'copy', 'close'],
    form: 'Fill the fields',
    hint: 'This snippet expects 2 values.',
    cancel: 'Cancel',
    copy: 'Copy filled',
    service: 'payment-api',
    namespace: 'staging',
  },
  drawing: {
    title: 'Network diagram',
    browser: 'Browser',
    balancer: ['Load', 'balancer'],
    api: ['Payment API 1', 'Payment API 2'],
    note: 'HTTPS ends here',
    exports: ['Export as PNG', 'Export as SVG'],
  },
  request: {
    title: 'List invoices',
    env: 'staging',
    host: 'https://api.staging.internal',
    path: '/invoices?status=unpaid',
    links: ['request', 'folder', 'collection'],
    found: '“staging” environment',
    send: 'Send',
    sending: 'Sending…',
    size: '2 B',
  },
  jwt: {
    tools: 'Tools ›',
    input: 'INPUT',
    header: 'HEADER',
    payload: 'PAYLOAD',
    load: 'eyJzdWIiOiJiaWxsaW5nIiwic2NvcGUiOiJpbnZvaWNlczpyZWFkIiwiaWF0IjoxNzk1NjkxODgwLCJleHAiOjE3OTU2OTU0ODB9',
    claims: ['"sub": "billing",', '"scope": "invoices:read"'],
    issued: 'Issued · iat',
    expires: 'Expires · exp',
    state: 'Expired · 12 minutes ago',
  },
  diff: {
    plus: '+ 4 lines',
    minus: '− 3 lines',
    was: 'A  nginx staging',
    now: 'B  nginx production',
  },
  search: {
    query: 'unpaid',
    count: '1 result',
    rows: [
      { title: 'Restart the payment service', where: 'Operations' },
      { title: 'Unpaid invoices for March', where: 'Projects' },
      { title: 'Before the demo', where: 'Projects' },
      { title: 'Trip budget', where: 'Getting started' },
      { title: 'Release procedure', where: 'Releases' },
    ],
    match: ['', 'Unpaid', ' invoices for March'],
    cite: ["WHERE status = '", 'unpaid', "'"],
  },
  calc: {
    title: 'Trip budget',
    train: 'train = 2 × 89 €',
    hotel: ['hotel = 3 × ', '112', '104', ' €'],
    meals: 'meals = 6 × 18 €',
    total: 'total = train + hotel + meals',
    results: ['178.00 €', '336.00 €', '312.00 €', '108.00 €', '622.00 €', '598.00 €'],
  },
  note: {
    title: 'Release procedure',
    tag: '#release',
    heading: 'Release',
    lead: ['Three steps, ', 'in this order', ':'],
    first: ['Freeze the ', 'dev', ' branch'],
    steps: ['Run the release pipeline', 'Warn the on-call engineer'],
    head: ['Step', 'Duration'],
    rows: [
      ['Build', '12 min'],
      ['Deployment', '4 min'],
    ],
    saved: 'Saved as Markdown',
    markdown:
      '# Release\nThree steps, **in this order**:\n1. Freeze the `dev` branch\n2. Run the release pipeline\n3. Warn the on-call engineer\n\n| Step       | Duration |\n| ---------- | -------- |\n| Build      | 12 min   |\n| Deployment | 4 min    |',
    present: 'Present',
  },
  lock: {
    library: 'Library “Work”',
    state: 'Encrypted on disk',
    rows: [
      ['Restart the payment service', '#operations', '1d ago'],
      ['Unpaid invoices for March', '#sql', '2d ago'],
      ['Trip budget', '#travel', '16:20'],
      ['Release procedure', '#release', '17:30'],
    ],
  },
};

export const SCENE_WORDS: Readonly<Record<Lang, SceneWords>> = { fr: FR, en: EN };
