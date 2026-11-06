// R666's provider buttons under `/login`'s forms, in one row (R3217000): Google's and Roblox's are drawn at
// one size (`OAUTH_BUTTON_WIDTH_PX` by `OAUTH_BUTTON_HEIGHT_PX`, set once in auth.css's
// `auth-oauth-branded` rule) so they read as a matched pair, and sit side by side, wrapping one above the
// other only when the screen is too narrow for both. A build that names one provider shows it alone,
// centered. The route owns what a click does (PKCE, R666; GIS's token, R2983000); this draws and reports.

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { OAUTH_PROVIDER_LABELS, type OAuthProvider } from "../net/auth.ts";
import GoogleButton from "./GoogleButton.tsx";
import { loginTestid } from "./testids.ts";

/** The brand mark's side, in pixels: the same for every provider's mark (R3217000). */
const OAUTH_MARK_PX = 18;

/** #1890: Google's four-colour "G", unaltered, as its sign-in branding guidelines ask of a Google button. */
function GoogleMark(): ReactElement {
  return (
    <svg className="auth-oauth-mark" viewBox="0 0 48 48" width={OAUTH_MARK_PX} height={OAUTH_MARK_PX} aria-hidden="true" focusable="false">
      <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z" />
      <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z" />
      <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z" />
      <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z" />
    </svg>
  );
}

/** The class names of one provider's button: Google's and Roblox's share `auth-oauth-branded`'s size (R3217000). */
function buttonClass(provider: OAuthProvider): string {
  switch (provider) {
    case "google":
      return "button-secondary auth-oauth-branded auth-oauth-google";
    case "custom:roblox":
      return "button-secondary auth-oauth-branded auth-oauth-roblox";
    default:
      return "button-secondary";
  }
}

export default function OAuthButtons({
  providers,
  googleClient,
  busy,
  onContinue,
  onGoogleCredential,
}: {
  providers: readonly OAuthProvider[];
  /** The Google client ID the build names, or null: GIS draws Google's own button only with one (R2983000). */
  googleClient: string | null;
  busy: boolean;
  onContinue: (provider: OAuthProvider) => void;
  onGoogleCredential: (idToken: string, nonce: string) => void;
}): ReactElement {
  return (
    <div className="auth-actions" data-testid={loginTestid.oauthRow}>
      {providers.map((provider) => {
        const button = (
          <button
            key={provider}
            type="button"
            className={buttonClass(provider)}
            data-testid={loginTestid.oauth}
            data-provider={provider}
            disabled={busy}
            onClick={() => {
              onContinue(provider);
            }}
          >
            {provider === "google" ? <GoogleMark /> : null}
            <span className="auth-oauth-label">
              {t("Continue with {provider}", { provider: OAUTH_PROVIDER_LABELS[provider] })}
            </span>
          </button>
        );
        // R2983000: Google's own button when the build names its client ID; R666's until it draws, or
        // when GIS cannot.
        return provider === "google" && googleClient !== null ? (
          <GoogleButton key={provider} clientId={googleClient} fallback={button} onCredential={onGoogleCredential} />
        ) : (
          button
        );
      })}
    </div>
  );
}
