import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import { securityHeaders } from "./headers.ts";

// The dev dashboard (#3074, README.md). Deployed, the page and `/api/*` share admin.jackioh.com's
// origin: the Worker (worker/index.ts) hands the allowed `/api/*` routes to the site's Worker. Locally,
// `vite` and `vite preview` stand in for it, proxying `/api` to the e2e server, so the page's requests
// are same-origin there too.

/** The dashboard's own port, beside the web's 5173 (apps/web/vite.config.ts). */
export const ADMIN_PORT = 5174;

/** The server `/api` reaches locally: the e2e server, `E2E=1 cargo run --release -p jackioh-server` (e2e/support/config.ts). */
export const LOCAL_SERVER = "http://localhost:8787";

const local = { port: ADMIN_PORT, strictPort: true, proxy: { "/api": LOCAL_SERVER } };

export default defineConfig({
  // `securityHeaders` fills the CSP of the build's `_headers` from its VITE_SUPABASE_URL (headers.ts).
  plugins: [react(), securityHeaders()],
  server: local,
  preview: local,
});
