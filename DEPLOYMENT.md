# Deed Landing Page - Deployment Guide

This guide will help you deploy the Deed landing page to Vercel for free.

## Quick Deploy to Vercel (5 minutes)

### Option 1: Deploy via Vercel Dashboard (Easiest)

1. **Push your code to GitHub** (already done if you're reading this)

2. **Go to Vercel**
   - Visit [vercel.com](https://vercel.com)
   - Sign in with your GitHub account

3. **Import Project**
   - Click "Add New..." → "Project"
   - Select the `Deed.so` repository
   - Click "Import"

4. **Configure Project**
   - Framework Preset: **Other** (or leave as detected)
   - Root Directory: `./` (leave as is)
   - Build Command: Leave empty
   - Output Directory: Leave empty
   - Click **Deploy**

5. **Done!**
   - Your site will be live at `your-project-name.vercel.app` in ~30 seconds
   - You can add a custom domain later in Vercel settings

### Option 2: Deploy via Vercel CLI

1. **Install Vercel CLI**
   ```bash
   npm i -g vercel
   ```

2. **Login to Vercel**
   ```bash
   vercel login
   ```

3. **Deploy**
   ```bash
   vercel --prod
   ```

4. **Follow the prompts:**
   - Set up and deploy? **Y**
   - Which scope? Select your account
   - Link to existing project? **N**
   - Project name? **deed-landing** (or your choice)
   - In which directory is your code? **./
   - Want to override settings? **N**

5. **Done!** Your site is live at the URL shown.

## Customizing the Closure Message

Before deploying, you should customize the closure message in `index.html`:

1. **Top Banner** (line ~26):
   ```html
   <span id="closure-message">This project is currently paused. Read more below.</span>
   ```

2. **Founder Message Section** (starting around line ~102):
   ```html
   <p id="founder-message">
       <!-- Replace with your personal message -->
   </p>
   ```

## Files Included

- `index.html` - Main landing page
- `styles.css` - Styling (dark theme, responsive)
- `script.js` - Smooth scrolling and animations
- `vercel.json` - Vercel configuration
- `.vercelignore` - Excludes Solana/Rust files from deployment

## Custom Domain Setup (Optional)

1. Go to your project in Vercel dashboard
2. Click "Settings" → "Domains"
3. Add your domain (e.g., `deed.so`)
4. Follow Vercel's instructions to update your DNS records

## Local Testing

To test locally before deploying:

1. Install a simple HTTP server:
   ```bash
   npm install -g http-server
   ```

2. Run it:
   ```bash
   http-server -p 8000
   ```

3. Open `http://localhost:8000` in your browser

## Support

For Vercel-specific issues, check:
- [Vercel Documentation](https://vercel.com/docs)
- [Vercel Community](https://github.com/vercel/vercel/discussions)

---

**Estimated Time:** 5-10 minutes total
**Cost:** $0 (Vercel's free tier is more than enough for a landing page)
