# Le client API

Envoyer des requêtes HTTP et GraphQL, ouvrir des WebSockets, les ranger en collections, et les faire varier d’un environnement à l’autre.

La zone *HTTP* (<kbd>Ctrl 3</kbd>) garde vos requêtes en collections, scellées comme vos notes. Une requête part vers le serveur que vous nommez, et nulle part ailleurs.

## Commencer une requête

*+ Nouvelle requête*, au bout des onglets ou dans le menu d’une collection, demande ce que vous commencez :

- **Requête HTTP** : une méthode, une URL, des en-têtes et un corps.
- **Requête GraphQL** : une requête et ses variables, envoyées en POST.
- **WebSocket** : une connexion ouverte, des messages dans les deux sens.

Chaque requête s’ouvre dans un onglet. <kbd>Ctrl Entrée</kbd> l’envoie (ou connecte le WebSocket), <kbd>Ctrl S</kbd> l’enregistre dans une collection. Les onglets se glissent pour changer d’ordre, et un clic du milieu en ferme un.

### Les parties d’une requête

- **Paramètres** et **En-têtes** : des tables clé / valeur, chaque ligne activable. Ceux que DevNotes ajoute de lui-même sont montrés à part.
- **Auth** : *Héritée*, *Aucune*, *Basic*, *Bearer* ou *Clé d’API*, envoyée dans un en-tête ou dans la query.
- **Corps** : JSON, texte, formulaire, multipart ou fichier. Un fichier est lu au moment de l’envoi, ses octets ne sont jamais gardés.
- **Réglages** : le délai d’attente, les redirections, la vérification des certificats TLS et les cookies.

### La réponse

Le statut, la durée et la taille, puis le corps *Formaté* ou *Brut* ; un JSON se lit aussi en *Graphe* ou en *Arbre*. À côté, les *En-têtes*, les *Cookies* posés et la *Chronologie* de l’échange. Une réponse s’enregistre dans un fichier ou se garde comme une note.

## Collections et dossiers

Une collection range des requêtes, éventuellement en dossiers. Ses *Réglages…* passent à ses requêtes : une requête dont l’auth est *Héritée* prend celle de son dossier ou de sa collection, et leurs en-têtes s’ajoutent aux siens.

**Dans le rail des collections**

| Touches | Action |
| --- | --- |
| <kbd>Alt ↑ ↓</kbd> | Déplacer une ligne parmi ses voisines |
| <kbd>Alt →</kbd> | La ranger dans le dossier juste au-dessus |
| <kbd>Alt ←</kbd> | La sortir de son dossier |
| <kbd>F2</kbd> | Renommer |
| <kbd>Suppr</kbd> | Supprimer, après confirmation |

## Variables et environnements

Une variable s’écrit entre doubles accolades, n’importe où dans la requête :

**Une URL qui varie selon l’environnement**

```
GET {{base_url}}/v1/invoices?status=paid
```

Elle est résolue au moment de l’envoi, du plus proche au plus lointain : la requête, son dossier, sa collection, l’environnement actif, puis les globales. Sous l’URL, chaque nom survolé montre sa valeur et d’où elle vient. Un nom que rien ne définit bloque l’envoi plutôt que de partir tel quel, et *Définir dans …* ouvre l’endroit où le définir.

1. *Gérer les environnements…* crée Local, Staging, Production : chacun un jeu de variables, avec une couleur.
2. L’environnement actif se choisit au bout des onglets. *Aucun* garde les globales seules.
3. Une variable est du *Texte*, un *Nombre* ou un *Secret* : un secret est masqué à l’écran et n’est jamais écrit dans l’historique.

> [!WARNING]
> Un environnement **protégé** demande avant d’envoyer autre chose qu’un GET, un HEAD ou un OPTIONS, et avant d’ouvrir un WebSocket. Mettez-le sur la production.

> [!TIP]
> Les `{{champs}}` d’un snippet et les variables d’une requête s’écrivent pareil, mais ne se mélangent pas : les premiers sont demandés à la copie, les secondes résolues à l’envoi.

Cinq noms sont fabriqués au départ de la requête, une valeur neuve à chaque fois : `{{$guid}}` (et `{{$randomUUID}}`), `{{$timestamp}}`, `{{$isoTimestamp}}` et `{{$randomInt}}`. Aucun niveau ne peut en définir un.

## Scripts et tests

Une requête, un dossier ou une collection peut exécuter du JavaScript *Avant l’envoi* et *Après la réponse*, et porter des *Tests* nommés, vérifiés après la réponse. Les scripts tournent dans un processus à part, sans accès à vos fichiers, et sont arrêtés s’ils sont trop longs ou trop gourmands en mémoire.

- Avant l’envoi, un script modifie la requête ou lit et écrit une variable : ajouter un en-tête, réécrire le corps, sauter la requête. Un script qui échoue arrête l’envoi, et le dit.
- Après la réponse, un script lit la réponse et en garde une valeur dans une variable, pour la requête suivante.
- Un test est une vérification nommée : son résultat va dans l’onglet *Tests*, avec ce qui était attendu et ce qui a été reçu.
- Les scripts de la collection passent en premier avant l’envoi et en dernier après la réponse, ceux de la requête étant les plus proches de la requête.

**Un test après la réponse**

```js
pm.test('Le statut est 200', () => {
  pm.response.to.have.status(200);
});
```

Les scripts parlent l’objet `pm` de Postman : variables, globales, requête et réponse, `pm.test`, `pm.expect` et `pm.sendRequest`. Le menu *Exemples* en insère quelques-uns à la fin du script.

### La console

La *Console* montre ce que la requête a envoyé, ce que les scripts impriment, et leurs erreurs. On la filtre, on la replie ou on la partage avec la réponse, et une ligne ouvre l’échange dont elle vient.

## Lancer une collection

Une collection ou un dossier se lance requête après requête, autant d’itérations que vous en demandez. Un fichier de données, CSV ou JSON, fournit une ligne de variables par itération. L’ordre des requêtes se change (glissez, ou <kbd>Alt ↑ ↓</kbd> sur la ligne), et <kbd>Espace</kbd> en laisse une de côté.

- *Arrêter au premier échec* met fin au lancement au premier test raté.
- *Garder les variables modifiées* et *Enregistrer les réponses* disent ce qui survit au lancement.
- Un environnement protégé demande avant que le lancement y change quoi que ce soit.

Les résultats listent chaque requête de chaque itération, avec ses échecs, et se filtrent.

## L’historique

Chaque envoi s’inscrit dans l’*Historique*, chiffré comme vos notes : 30 jours, 500 envois au plus. Un envoi se relit, se rouvre dans sa requête ou en brouillon, et s’efface. Les secrets y sont masqués avant d’être écrits.

## Le code d’une requête

Le bouton *</> Code* écrit la requête pour vingt cibles, de cURL et PowerShell à fetch, axios ou le HttpClient d’Angular. Les variables peuvent y être résolues ; les secrets restent en variables, sauf si vous demandez à les inclure.

## Les WebSockets

Un WebSocket se connecte, puis échange : le journal montre chaque message envoyé et reçu, avec son heure. Les messages se gardent pour être renvoyés. Changer de bibliothèque ferme les connexions ouvertes.
