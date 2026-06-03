# EasySpecy Documentation — Quick Reference

## 📂 File Locations

| File | Purpose |
|------|---------|
| `docs/.vitepress/config.ts` | VitePress configuration (sidebar, nav, theme) |
| `docs/.vitepress/theme/custom.css` | EasySpecy brand system (colors, typography, animations) |
| `docs/index.md` | Landing page (hero + features) |
| `.github/workflows/deploy-docs.yml` | GitHub Pages deployment workflow |

## 🚀 Commands

```bash
# Start dev server (hot reload on localhost:5173)
npm run docs:dev

# Build for production
npm run docs:build

# Preview production build locally
npm run docs:preview
```

## 🎨 Brand Colors

### Dark Mode (Primary)
```css
--brand-surface-base: #0d0f1a;
--brand-surface-raised: #151828;
--brand-accent: #00e88a;
--brand-text-primary: #eef0f6;
--brand-text-secondary: #9a9eb5;
```

### Light Mode (Accessibility)
```css
--brand-surface: #f4fbf2;
--brand-accent: #006e3e;
--brand-text: #2d342e;
```

## 📝 Documentation Structure

```
docs/
├── index.md                  ← Landing page
├── guide/                    ← User guides
│   ├── introduction.md
│   ├── installation.md
│   └── quick-start.md
├── api/                      ← API reference (TODO)
├── config/                   ← Configuration docs (TODO)
└── advanced/                 ← Advanced topics (TODO)
```

## ✅ Deployment Checklist

- [ ] GitHub Pages enabled (Settings → Pages → GitHub Actions)
- [ ] `base` URL correct in `config.ts` (`/EasySpecy/`)
- [ ] Logo copied to `docs/public/logo.png`
- [ ] Pushed to `main` branch
- [ ] Workflow completed successfully (Actions tab)
- [ ] Site live at `https://IsNoobgrammer.github.io/EasySpecy/`

## 🔧 Troubleshooting

| Issue | Fix |
|-------|-----|
| 404 on GitHub Pages | Check `base` URL in config.ts |
| Build fails | Run `npm ci` locally to check for errors |
| Assets not loading | Ensure paths start with `/` |
| Workflow not triggering | Push to `main` with changes in `docs/**` |

## 📖 Writing New Docs

1. Create `.md` file in appropriate directory
2. Add to sidebar in `config.ts`
3. Use this template:

```markdown
# Page Title

Brief description.

## Section

Content here.

### Subsection

More content.

**Code example:**

```bash
npm run docs:dev
```

::: tip
Pro tip here!
:::
```

## 🎯 Next Steps

- [ ] Complete all guide pages (screen-capture, audio, etc.)
- [ ] Write API reference documentation
- [ ] Add configuration docs
- [ ] Create advanced topics (architecture, A/V sync)
- [ ] Add video tutorials
- [ ] Set up Algolia DocSearch

---

**Built with:** VitePress 1.6+
**Theme:** EasySpecy Brand System
**Deployed:** GitHub Pages via GitHub Actions
