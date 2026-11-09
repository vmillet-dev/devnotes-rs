# Données de la recette 0.16.0

Fichiers cités par la feuille de recette (colonne « Fichier »). Aucun ne contient de vraie donnée : les secrets sont des valeurs de test (`secret`, `recette-devnotes-2026`).

Pour mettre un fichier tel quel dans le presse-papier, fins de ligne et caractères invisibles compris :

```powershell
Get-Content -Raw recette\donnees\fins-de-ligne-mixtes.txt | Set-Clipboard
```

| Fichier                                                                             | Sert à                                                                                                                      |
| ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `json-facture.json`, `json-facture-v2.json`                                         | explorateur JSON, Diff « Données » (4 modifications, 1 ajout, 1 suppression), convertisseur                                 |
| `json-invalide.json`                                                                | erreur de lecture (virgule manquante ligne 3, signalée ligne 4)                                                             |
| `json-mal-indente.json`                                                             | formatage Prettier                                                                                                          |
| `config.yaml`, `config.toml`, `config.xml`                                          | explorateurs YAML/TOML/XML, convertisseur, Diff                                                                             |
| `schema-utilisateur.json`, `modele-commande.json`                                   | générateur JSON, modes « Schéma » et « Modèle »                                                                             |
| `texte-a.txt`, `texte-b.txt`                                                        | Diff « Texte » (+2 lignes, −2 lignes)                                                                                       |
| `fins-de-ligne-mixtes.txt`                                                          | Sauts de ligne (CRLF, LF, CR, espaces en fin de ligne, pas de saut final)                                                   |
| `caracteres-invisibles.txt`                                                         | Statistiques du texte (espace insécable, espace fine insécable, espace sans chasse, tabulation, trait d’union conditionnel) |
| `image-test.png`, `image-test-data-uri.txt`                                         | pièces jointes, Base64 (fichier et aperçu d’image), Hash d’un fichier                                                       |
| `gros-fichier-11-Mo.bin`                                                            | refus d’une pièce jointe de plus de 10 Mo                                                                                   |
| `snippet-2000-lignes.ts`                                                            | performance de l’éditeur de code                                                                                            |
| `snippet-champs.sh`                                                                 | champs `{{…}}` avec et sans valeur par défaut                                                                               |
| `note-presentation.md`                                                              | Note mise en forme, Mermaid, « Présenter » (couverture + 6 diapositives : « Diapositive 1 sur 7 »)                          |
| `mermaid-invalide.txt`                                                              | diagramme qui ne se lit pas                                                                                                 |
| `checklist-a-coller.txt`                                                            | collage de plusieurs lignes dans une liste de tâches                                                                        |
| `headers-a-coller.txt`                                                              | collage de plusieurs en-têtes HTTP                                                                                          |
| `feuille-calcul.txt`                                                                | feuille Calcul complète, résultat attendu en commentaire de chaque ligne                                                    |
| `runner-donnees.csv`, `runner-donnees.json`, `runner-invalide.txt`                  | exécution d’une collection avec un fichier de données                                                                       |
| `script-*.js`                                                                       | scripts HTTP avant l’envoi, après la réponse, tests, boucle infinie, `require`, `pm.sendRequest`                            |
| `graphql-pays.graphql`, `graphql-variables.json`, `graphql-deux-operations.graphql` | requêtes GraphQL sur `https://countries.trevorblades.com/graphql`                                                           |
| `export-pas-devnotes.devnotes`                                                      | import refusé d’un fichier qui n’est pas un export                                                                          |
