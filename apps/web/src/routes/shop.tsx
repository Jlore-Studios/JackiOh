// `/shop`: the Jlore shop on its own screen (#1577, SPEC §14.1, §14.3). Until the Shop part
// reshapes this hub, it renders the same panel the account screen shows behind its packs toggle
// (`routes/account.tsx`), so the two read the server per mount and neither decides anything
// (CLAUDE.md rule 7). Gated like `/collection`: an anonymous visitor goes to `/login` (main.tsx).

import { Suspense, lazy, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./account.css";

// The shop, its token picker and its cosmetic packs ride one chunk, as they do beside the pack
// opening on the account screen.
const Shop = lazy(() => import("../shop/Shop.tsx"));

export default function ShopRoute({ token }: { token: string }): ReactElement {
  return (
    <div className="app-shell tavern shop-screen">
      <BackLink />
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Shop")}</h2>
        <Suspense fallback={null}>
          <Shop token={token} />
        </Suspense>
      </section>
    </div>
  );
}
