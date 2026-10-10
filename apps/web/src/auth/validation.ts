// Loose sign-in, sign-up and reset checks (B24, B31): the provider stays authoritative and
// `classifyProviderRefusal` maps its errors. Password bounds are public named constants
// (`crates/server/src/config.rs`, CLAUDE.md rule 9).

import { AUTH_PASSWORD_MAX_LENGTH, AUTH_PASSWORD_MIN_LENGTH } from "@jackioh/server-config";

const EMAIL_SHAPE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/u;

/** Trim only. Case is left alone: the provider folds it, and the address is shown back as typed. */
export function normalizeEmail(raw: string): string {
  return raw.trim();
}

export function emailProblem(raw: string): string | null {
  const email = normalizeEmail(raw);
  if (email.length === 0) return "Enter your email address.";
  if (!EMAIL_SHAPE.test(email)) return "Enter a valid email address, like name@example.com.";
  return null;
}

/** A password's length as the provider measures its limit: UTF-8 bytes, not characters. */
export function passwordBytes(password: string): number {
  return new TextEncoder().encode(password).length;
}

/** Sign-up/reset only: byte upper bounds are not character counts, while character lower bounds fit. */
export function newPasswordProblem(password: string): string | null {
  if (password.length < AUTH_PASSWORD_MIN_LENGTH) {
    return `Use at least ${String(AUTH_PASSWORD_MIN_LENGTH)} characters.`;
  }
  if (passwordBytes(password) > AUTH_PASSWORD_MAX_LENGTH) {
    return `That password is too long. Use a shorter one: the limit is ${String(AUTH_PASSWORD_MAX_LENGTH)} bytes, and a letter outside A–Z takes two or more.`;
  }
  return null;
}

export function confirmProblem(password: string, confirm: string): string | null {
  if (confirm.length === 0) return "Type the new password again.";
  if (confirm !== password) return "The two passwords don't match.";
  return null;
}

export function requiredProblem(value: string, label: string): string | null {
  if (value.trim().length === 0) return `Enter your ${label.toLowerCase()}.`;
  return null;
}
