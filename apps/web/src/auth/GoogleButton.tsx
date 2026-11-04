// R2983000: Google's own "Sign in with Google" button on `/login`, drawn by Google Identity Services
// (`googleIdentity.ts`) so that Google's screen names this site. Each attempt is armed with a fresh
// nonce, whose raw value only this component's closure holds until it hands it to `onCredential`.
// R666's button (`fallback`) shows while GIS loads and stays whenever GIS cannot draw: its script does
// not load, is blocked, has no `google.accounts.id`, or throws, or this browser cannot hash a nonce.

import { useEffect, useLayoutEffect, useRef, useState, type ReactElement } from "react";

import { GIS_BUTTON, loadGoogleIdentity, newGoogleNonce } from "./googleIdentity.ts";
import { loginTestid } from "./testids.ts";

export default function GoogleButton({
  clientId,
  fallback,
  onCredential,
}: {
  clientId: string;
  fallback: ReactElement;
  /** The ID token GIS returned, with the raw nonce whose SHA-256 it was armed with. */
  onCredential: (idToken: string, nonce: string) => void;
}): ReactElement {
  const slot = useRef<HTMLDivElement>(null);
  const [drawn, setDrawn] = useState(false);
  const [failed, setFailed] = useState(false);
  const latest = useRef(onCredential);

  useLayoutEffect(() => {
    latest.current = onCredential;
  });

  useEffect(() => {
    let live = true;
    const fail = (): void => {
      if (live) setFailed(true);
    };
    const arm = async (): Promise<void> => {
      const api = await loadGoogleIdentity();
      const nonce = await newGoogleNonce();
      const parent = slot.current;
      if (!live || parent === null) return;
      api.initialize({
        client_id: clientId,
        nonce: nonce.hashed,
        context: "signin",
        ux_mode: "popup",
        use_fedcm_for_button: true,
        callback: (response) => {
          if (!live) return;
          if (typeof response.credential === "string") latest.current(response.credential, nonce.raw);
          // R2983000: the next attempt gets a new nonce.
          void arm().catch(fail);
        },
      });
      parent.replaceChildren();
      api.renderButton(parent, GIS_BUTTON);
      setDrawn(true);
    };
    void arm().catch(fail);
    return () => {
      live = false;
    };
  }, [clientId]);

  return (
    <>
      {failed ? null : <div ref={slot} className="auth-google" data-testid={loginTestid.googleButton} />}
      {drawn && !failed ? null : fallback}
    </>
  );
}
