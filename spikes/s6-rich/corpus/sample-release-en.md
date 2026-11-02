A Note is written formatted and stored as Markdown: headings, tasks, tables, code blocks.

## Before
- [x] Freeze the merges on `main`
- [ ] Warn the support team
- [ ] Back the database up

## Where
| Environment | Address | Who |
| --- | --- | --- |
| Staging | staging.example.com | API team |
| Production | app.example.com | On call |

## Check
```bash
kubectl rollout status deploy/billing --timeout=120s
curl -fsS https://app.example.com/health
```