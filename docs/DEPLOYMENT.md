# Deploying EasySpecy Documentation to GitHub Pages

This guide walks you through deploying the EasySpecy documentation site to GitHub Pages.

## Prerequisites

- GitHub repository with EasySpecy codebase
- Push access to the `main` branch
- GitHub Pages enabled on the repository

## Step 1: Enable GitHub Pages

1. Go to your repository on GitHub: `https://github.com/IsNoobgrammer/EasySpecy`
2. Click **Settings** tab
3. In the left sidebar, click **Pages**
4. Under **Source**, select **GitHub Actions** (NOT "Deploy from a branch")
5. Click **Save**

## Step 2: Verify Workflow File

Ensure the deployment workflow exists at:

```
.github/workflows/deploy-docs.yml
```

This file is already created and configured. It will:
- Trigger on pushes to `main` that modify `docs/**`
- Build the VitePress site
- Deploy to GitHub Pages automatically

## Step 3: Configure Base URL (If Needed)

If your repository is NOT at `https://github.com/IsNoobgrammer/EasySpecy`, update the base URL:

**File:** `docs/.vitepress/config.ts`

```typescript
export default defineConfig({
  base: '/YourRepoName/',  // Change this to match your repo name
  // ... rest of config
})
```

**Examples:**
- `https://IsNoobgrammer.github.io/EasySpecy/` → `base: '/EasySpecy/'`
- `https://YourUsername.github.io/MyApp/` → `base: '/MyApp/'`
- Custom domain → `base: '/'`

## Step 4: Push to Main Branch

```bash
# Add all documentation files
git add docs/ .github/workflows/deploy-docs.yml package.json

# Commit
git commit -m "docs: add VitePress documentation site with GitHub Pages deployment"

# Push to main branch
git push origin main
```

## Step 5: Monitor Deployment

1. Go to **Actions** tab in your GitHub repository
2. You should see "Deploy Documentation" workflow running
3. Click on the workflow run to view logs
4. Wait for build and deploy jobs to complete (usually 2-3 minutes)

### Expected Workflow Output

```
✅ Checkout
✅ Setup Node.js
✅ Install dependencies
✅ Build documentation
✅ Setup Pages
✅ Upload artifact
✅ Deploy to GitHub Pages
```

## Step 6: Verify Live Site

After deployment completes:

1. Go to **Settings** → **Pages**
2. You'll see: "Your site is live at `https://IsNoobgrammer.github.io/EasySpecy/`"
3. Click the link to visit your documentation site

## Step 7: Test the Site

Visit the live site and verify:

- [ ] **Homepage** loads with hero section and features
- [ ] **Navigation** works (Guide, API, Config, GitHub links)
- [ ] **Sidebar** shows all sections
- [ ] **Search** works (Cmd+K / Ctrl+K)
- [ ] **Dark mode** is default and looks correct
- [ ] **Light mode** toggle works
- [ ] **Code blocks** have syntax highlighting and copy buttons
- [ ] **Mobile responsive** — test on phone or resize browser
- [ ] **All links** work (no 404s)
- [ ] **Logo** displays in navbar and hero

## Troubleshooting

### Issue: 404 on GitHub Pages

**Cause:** Incorrect `base` URL in VitePress config

**Fix:**
```typescript
// docs/.vitepress/config.ts
export default defineConfig({
  base: '/EasySpecy/',  // Must match your repo name
})
```

### Issue: Assets Not Loading

**Cause:** Asset paths not relative to base URL

**Fix:** Ensure all asset paths start with `/`:
```markdown
![Logo](/logo.png)  # ✅ Correct
![Logo](logo.png)   # ❌ Wrong
```

### Issue: Build Fails

**Cause:** Missing dependencies or Node version mismatch

**Fix:**
1. Check workflow logs for specific error
2. Ensure `vitepress` is in `devDependencies`
3. Verify Node.js version is 18+

### Issue: Search Not Working

**Cause:** Search index not built

**Fix:** Search is built-in with VitePress local search. Ensure:
```typescript
// docs/.vitepress/config.ts
themeConfig: {
  search: {
    provider: 'local'
  }
}
```

### Issue: Workflow Not Triggering

**Cause:** Push not to `main` branch, or no changes to `docs/**`

**Fix:**
1. Push to `main` branch
2. Ensure changes are in `docs/` directory
3. Or trigger manually: Actions → Deploy Documentation → Run workflow

## Custom Domain (Optional)

If you want to use a custom domain (e.g., `docs.easyspecy.com`):

1. **Add CNAME file:**

```bash
echo "docs.easyspecy.com" > docs/public/CNAME
```

2. **Configure DNS:**

Add a CNAME record pointing to `IsNoobgrammer.github.io`

3. **Update VitePress config:**

```typescript
export default defineConfig({
  base: '/',  # Root path for custom domain
  // ...
})
```

4. **Enable custom domain in GitHub:**

Settings → Pages → Custom domain → Enter your domain → Save

## Updating Documentation

After initial deployment, updating is automatic:

```bash
# Make changes to docs
git add docs/
git commit -m "docs: update installation guide"
git push origin main
```

GitHub Actions will automatically rebuild and deploy.

## Performance Optimization

The workflow is already optimized with:

- ✅ **npm ci** instead of npm install (faster, deterministic)
- ✅ **npm cache** (speeds up subsequent builds)
- ✅ **Path filtering** (only triggers on docs changes)
- ✅ **Separate build/deploy jobs** (better error isolation)
- ✅ **Concurrency control** (prevents duplicate deployments)

## Analytics (Optional)

To add analytics, edit `docs/.vitepress/config.ts`:

```typescript
head: [
  // Google Analytics
  ['script', { async: '', src: 'https://www.googletagmanager.com/gtag/js?id=YOUR_ID' }],
  ['script', {}, `window.dataLayer = window.dataLayer || [];
    function gtag(){dataLayer.push(arguments);}
    gtag('js', new Date());
    gtag('config', 'YOUR_ID');`],
]
```

## Next Steps

- [ ] Add video tutorials to guides
- [ ] Set up Algolia DocSearch for better search
- [ ] Enable versioned documentation (v0.1, v0.2)
- [ ] Add community-contributed examples
- [ ] Set up i18n for multiple languages

---

**Documentation live at:** `https://IsNoobgrammer.github.io/EasySpecy/`
**Built with:** VitePress + EasySpecy Brand System
**Deployed via:** GitHub Actions
