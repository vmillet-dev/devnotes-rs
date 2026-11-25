# Les outils

Une trentaine d’utilitaires qui tournent sur votre machine sans rien appeler : encoder, hacher, convertir, générer, lire une date, calculer, et des références.

La zone *Outils* (<kbd>Ctrl 2</kbd>) les range par catégorie. Aucun n’appelle le réseau : ce que vous collez ne quitte pas l’application.

## Trouver un outil

- <kbd>Ctrl Shift T</kbd> cherche un outil depuis n’importe quelle zone, et fouille aussi les références.
- Le champ d’un outil peut aussi se remplir depuis une note, un snippet ou une liste de tâches.
- Une étoile met un outil dans les *Favoris* ; les derniers ouverts sont dans les *Récents*.
- Le bouton *Exemple* d’un outil le remplit pour voir ce qu’il fait.
- Un résultat se copie tel qu’il est affiché, ou *Brut* quand les deux diffèrent.

## Garder un résultat

*Enregistrer comme note* range ce que l’outil a calculé dans un snippet, avec un titre, un espace, un dossier et des tags. Jamais ce qui y a été tapé : un secret collé dans un outil n’atterrit pas dans vos notes par mégarde.

## Tous les outils

**Texte**

| Outil | Ce qu’il fait |
| --- | --- |
| **Convertisseur de casse** | camelCase, snake_case, dot.case, Train-Case… et le slug |
| **Sauts de ligne** | CRLF ↔ LF, espaces en fin de ligne |
| **Statistiques du texte** | caractères, mots, lignes, octets, fréquences |
| **Diff** | deux textes ligne à ligne, ou deux documents JSON, YAML, TOML ou XML comparés par leurs valeurs |

**Crypto**

| Outil | Ce qu’il fait |
| --- | --- |
| **Hash / HMAC** | MD5, SHA-1, SHA-2, SHA-3 |
| **Mots de passe** | longueur, jeux de caractères, entropie |
| **Identifiants** | UUID, ULID, NanoID, CUID2, ObjectId, KSUID, tirés et relus |
| **JWT** | décodé, ses dates lues, sa signature HS\* vérifiée |

**Encodeurs**

| Outil | Ce qu’il fait |
| --- | --- |
| **URL** | encoder, décoder, analyser une adresse et ses paramètres |
| **Base64 et autres bases** | Base64, Base32, hexadécimal, binaire, octets ; l’aperçu d’une image |
| **JSON ↔ YAML ↔ TOML ↔ XML** | d’un format à l’autre, dans tous les sens |
| **Couleurs** | HEX, RGB, HSL, OKLCH, et le contraste entre deux couleurs |

**Générateurs**

| Outil | Ce qu’il fait |
| --- | --- |
| **JSON** | des données de test, à partir d’un schéma ou d’un document modèle |
| **Identité fictive** | une personne de test dont chaque numéro passe sa clé |
| **Lorem ipsum** | mots, phrases, paragraphes |
| **QR code** | texte, URL, Wi-Fi, e-mail ou contact, en SVG ou PNG |

**Temps**

| Outil | Ce qu’il fait |
| --- | --- |
| **Dates et durées** | Unix ↔ ISO 8601, écarts et durées ISO |
| **Fuseaux horaires** | une heure, dans chaque fuseau choisi |
| **Cron** | lu à voix haute, ses prochaines exécutions, et celles entre deux moments |

**Calcul**

| Outil | Ce qu’il fait |
| --- | --- |
| **Tailles et transfert** | o, ko, Mo, Go ↔ Kio, Mio, Gio, les bits, et le temps d’un transfert |
| **Pourcentages** | x % de y, part, évolution, hausse et baisse |
| **Permissions Unix** | chmod : 755 ↔ rwxr-xr-x, bits spéciaux, umask |
| **Luhn et IBAN** | un numéro ou un IBAN vérifié, sa bonne clé, et des numéros valides à générer |
| **Sous-réseaux CIDR** | masques, hôtes, découpage, plan d’adressage, IPv4 et IPv6 |

**Références**

| Référence | Ce qu’elle contient |
| --- | --- |
| **Codes de statut HTTP** | 404, 429, 503 : ce qu’ils veulent dire, et quand les renvoyer |
| **En-têtes HTTP** | Content-Type, Cache-Control, CORS, CSP : à quoi ils servent |
| **Types MIME** | le type d’une extension, et les extensions d’un type |
| **Ports courants** | 22, 443, 5432, 6379 : quel service écoute où |
| **Signaux Unix** | SIGTERM, SIGKILL, SIGHUP : numéros, action et usage |
| **Table ASCII** | les 128 caractères, en décimal, hexadécimal, octal et binaire |
| **Aide-mémoire regex** | la syntaxe, en JavaScript et en PCRE |
| **Aide-mémoire Markdown** | titres, listes, liens, tableaux, en CommonMark et sur GitHub |
| **Aide-mémoire Mustache** | variables, sections, partiels, avec ce qu’ils rendent |

> [!TIP]
> Un outil qui ouvre un fichier le lit jusqu’à 10 Mo. Au-delà, il le dit plutôt que de le tronquer en silence.
