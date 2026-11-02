Une Note s’écrit mise en forme et s’enregistre en Markdown : titres, tâches, tableaux, blocs de code.

## Avant
- [x] Geler les merges sur `main`
- [ ] Prévenir le support
- [ ] Sauvegarder la base

## Où
| Environnement | Adresse | Qui |
| --- | --- | --- |
| Préproduction | staging.exemple.fr | Équipe API |
| Production | app.exemple.fr | Astreinte |

## Vérifier
```bash
kubectl rollout status deploy/billing --timeout=120s
curl -fsS https://app.exemple.fr/health
```