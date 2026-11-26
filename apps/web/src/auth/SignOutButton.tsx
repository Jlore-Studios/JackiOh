// The gate panels' Sign out link (the username prompt's and the consent prompt's), which says so while a
// sign-out waits on its revocation (`signOut.ts`). Like `signOut.ts`, it rides the entry chunk.

import type { ReactElement } from "react";

import { signOut, signOutLabel, useSigningOut } from "./signOut.ts";
import { shellTestid } from "./testids.ts";

export function SignOutButton(): ReactElement {
  const leaving = useSigningOut();
  return (
    <button
      type="button"
      className="link-button"
      data-testid={shellTestid.signOut}
      disabled={leaving}
      aria-busy={leaving}
      onClick={() => {
        signOut();
      }}
    >
      {signOutLabel(leaving)}
    </button>
  );
}
