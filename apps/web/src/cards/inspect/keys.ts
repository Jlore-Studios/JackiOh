// The keys that open a card's inspect view from the keyboard: I, the context-menu key, and
// Shift+F10. The deck builder's tiles (DeckSidebar.tsx) and the board's cards (game/Card.tsx, #258)
// answer to the same three.

export const INSPECT_KEY_SHORTCUT = "I";

export function isInspectKey(event: { key: string; shiftKey: boolean }): boolean {
  return event.key === "i" || event.key === "I" || event.key === "ContextMenu" || (event.shiftKey && event.key === "F10");
}
