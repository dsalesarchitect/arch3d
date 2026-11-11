# Arch3D Landing Page — Deployment Guide

This guide details how to host and deploy the Arch3D landing page to **GitHub Pages** and **Cloudflare Pages**.

The landing page assets live entirely in the [`docs/`](./docs) directory and are 100% static (HTML, CSS, Vanilla JS), with **zero npm build dependencies**, instant edge caching, and mobile responsiveness.

---

## 1. Hosting on GitHub Pages

There are two easy methods to deploy on GitHub Pages:

### Method A: Automated GitHub Actions (Recommended)
Arch3D includes an automated GitHub Actions deployment workflow at [`.github/workflows/deploy-pages.yml`](./.github/workflows/deploy-pages.yml).

1. In your GitHub repository, navigate to **Settings** → **Pages**.
2. Under **Build and deployment** → **Source**, select **GitHub Actions**.
3. Push any commit to the `main` or `master` branch (or manually trigger the action from the **Actions** tab).
4. The workflow will automatically deploy `docs/` to `https://<your-username>.github.io/arch3d/`.

### Method B: Deploy from Branch (`/docs` folder)
If you prefer not using GitHub Actions:
1. In your GitHub repository, navigate to **Settings** → **Pages**.
2. Under **Build and deployment** → **Source**, select **Deploy from a branch**.
3. Under **Branch**, select `main` (or `master`) and change the folder dropdown from `/ (root)` to **`/docs`**.
4. Click **Save**. GitHub Pages will be live within 60 seconds.

> [!NOTE]
> The [`docs/.nojekyll`](./docs/.nojekyll) file ensures GitHub Pages serves all assets directly without Jekyll processing.

---

## 2. Hosting on Cloudflare Pages

Cloudflare Pages provides global CDN edge distribution, automatic SSL/TLS, enterprise DDoS protection, and instant invalidation.

### Method A: Cloudflare Dashboard (Connected to Git)
1. Log in to the [Cloudflare Dashboard](https://dash.cloudflare.com/) and navigate to **Workers & Pages**.
2. Click **Create Application** → **Pages** → **Connect to Git**.
3. Select your repository (`arch3d`).
4. In the **Set up builds and deployments** section:
   - **Framework preset**: `None`
   - **Build command**: *(leave completely blank)*
   - **Build output directory**: `docs`
   - **Root directory**: `/`
5. Click **Save and Deploy**. Your site will be live at `https://arch3d.pages.dev` in less than 15 seconds.

### Method B: 10-Second Command Line Deploy via Wrangler CLI
You can also deploy directly from your local terminal using Cloudflare's Wrangler CLI without touching Git:

```bash
# Authenticate with Cloudflare (first time only)
npx wrangler login

# Deploy the docs directory
npx wrangler pages deploy docs --project-name arch3d
```

The [`wrangler.toml`](./wrangler.toml) file is already preconfigured in the repository root.

---

## 3. Cloudflare Security Headers & Redirects

The following Cloudflare-specific rules are pre-configured in `docs/`:

- **[`docs/_headers`](./docs/_headers)**:
  - Strict Transport Security (`HSTS max-age=31536000`)
  - Content Security Policy (`CSP`)
  - Frame options (`SAMEORIGIN`)
  - 1-year immutable caching for static assets (`styles.css`, `app.js`, `.ico`, `.png`, `.svg`)
  - Instant revalidation for `index.html` updates
- **[`docs/_redirects`](./docs/_redirects)**:
  - Clean redirects for `/home`, `/download`, `/features`, `/specs`, and `/docs`

---

## 4. Custom Domain Setup (Optional)

1. In the **Cloudflare Pages** dashboard for `arch3d`, go to **Custom domains** → **Set up a custom domain**.
2. Enter your domain (e.g. `arch3d.io` or `bim.yourdomain.com`).
3. Cloudflare will automatically provision free SSL certificates and route traffic to the global edge network.
4. If using GitHub Pages with a custom domain, add your domain name to [`docs/CNAME`](./docs/CNAME).

---

## 5. Local Testing

To preview the landing page locally before deploying:

```bash
# Using Python built-in HTTP server:
cd d:\Arch3D\docs
python -m http.server 8080

# Or using npx serve:
npx serve docs
```

Then open `http://localhost:8080` in your browser.
