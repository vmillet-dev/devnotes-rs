# Questions fréquentes

Le prix, macOS, la synchronisation, une phrase de passe oubliée, l’avertissement de Windows, et où vont vos données.

## DevNotes est-il gratuit ?

Oui, et libre : il est publié sous licence GPL-3.0, son code est sur [GitHub](https://github.com/vmillet-dev/devnotes-rs). Pas de compte, pas d’abonnement, pas de publicité.

## Y a-t-il une version macOS ?

Pas pour l’instant. Elle ne peut pas être testée ici, et Gatekeeper réclame un compte Apple Developer payant, sans contournement gratuit sur les versions récentes.

## Puis-je synchroniser deux ordinateurs ?

DevNotes ne synchronise rien : c’est ce qui garantit que vos notes ne quittent pas votre machine. Pour passer des notes d’un poste à l’autre, exportez-les dans une archive protégée, puis importez-la de l’autre côté. Un import ne crée jamais de doublon.

## J’ai oublié ma phrase de passe.

Elle ne se récupère pas : elle enveloppe la clé qui scelle vos notes, et rien d’autre ne l’ouvre. *Phrase de passe oubliée ?* archive la bibliothèque, toujours scellée, et en commence une nouvelle. Un export en clair gardé à part est la seule copie qui n’en dépend pas.

## Windows affiche un avertissement au premier lancement.

Choisissez **Informations complémentaires**, puis **Exécuter quand même**. L’avertissement porte sur l’identité de l’éditeur : DevNotes n’a pas de certificat de signature de code. Chaque version publie `SHA256SUMS.txt` pour vérifier les fichiers, et les mises à jour sont signées et vérifiées avant d’être installées.

## DevNotes marche-t-il hors ligne ?

Entièrement. Seuls le client API, qui contacte les serveurs que vous lui donnez, et la recherche de mises à jour ont besoin du réseau.

## Comment mettre à jour ?

DevNotes propose lui-même les nouvelles versions, avec leurs notes, et les installe pour l’installateur Windows et l’AppImage. Un `.deb` ou un `.rpm` est mis à jour par le gestionnaire de paquets de votre distribution. Vous pouvez aussi télécharger [la dernière version](https://devnotes.fr/telecharger) et l’installer par-dessus : vos bibliothèques ne sont pas touchées.

## Mes bibliothèques d’une version précédente s’ouvriront-elles ?

Oui, telles quelles. DevNotes est désormais une application native : aucun moteur de navigateur à l’intérieur, donc rien comme WebView2 ou WebKitGTK à installer.

## Sous Linux avec Wayland, il manque quelque chose.

DevNotes utilise X11 dès qu’il est là, XWayland compris. Dans une session Wayland sans XWayland, ce qui est rare, la position de la fenêtre n’est pas restaurée, fermer dans la barre système ne fait que réduire, un fichier ne se dépose pas depuis le gestionnaire de fichiers, et un HTML collé dans une Note n’en garde que le texte.

## Où sont mes notes ?

Dans le dossier de votre profil : `%APPDATA%\com.devnotes.app\` sous Windows, `~/.local/share/com.devnotes.app/` sous Linux. Le détail est dans [Sécurité et sauvegardes](safety.md).

## J’ai trouvé un bug, ou j’ai une idée.

Ouvrez un ticket sur [le dépôt GitHub](https://github.com/vmillet-dev/devnotes-rs/issues), en disant ce que vous faisiez, ce que vous attendiez et ce qui s’est passé. La version est dans *À propos*.

> [!TIP]
> Rien de ce que vous tapez dans DevNotes n’est envoyé avec un ticket : copiez seulement ce qui aide à comprendre.

## Ctrl Alt P ne fait rien.

Une autre application tient sans doute déjà cette combinaison : DevNotes le signale au démarrage. Choisissez-en une autre dans *Préférences… → Raccourcis*, ou libérez-la dans l’autre application. Les raccourcis globaux marchent fenêtre fermée tant que DevNotes tourne dans la barre système ; une fois *Quitter DevNotes* choisi, <kbd>Ctrl Alt P</kbd> n’a plus d’application à réveiller.
