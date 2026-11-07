// One dialog (#3220, R3220000): the board's confirms and the practice result draw through it, and so do
// the menus' dialogs and sheets (#3221, R3221000). It decides nothing (CLAUDE.md rule 7): each answer is
// its caller's.
//
// - A modal `dialog` or `alertdialog`, labelled by its `h2` title and described by its line when it has one.
// - Close: a `dialog` closes by its ✕ at the top right, named "Close"; an `alertdialog` has no ✕, its
//   Cancel (the safe choice) is the close.
// - Order: the safe choice first, holding the focus from the start, so a stray Enter or Space never
//   acts; the act last, at the right, and a `danger` answer always last. A third answer goes between,
//   and the safe choice then takes the first row (R765). The row stacks in the same order at 360 px.
// - Keys: `useModalKeys`'s. Escape, Back (a layer, R2127000) and a click on the scrim close it (one
//   whose press began in the panel does not, R3221000), Tab
//   stays inside, and the focus goes back to the opener unless the caller has moved it. It takes no
//   key handler of its own (R2126000).
// - Layer: `--z-dialog` (surfaces.css), over the prompt's scrim, the result overlay, the drag layer and
//   the toasts: it is the one question on the screen while it is open.
// - Sheet (R3221000): `sheet` keeps the panel centred on a wide screen and puts it at the foot of an
//   upright phone and down the right edge of a phone on its side (dialog.css); keys and close are the same.
// - `DialogClose` is the ✕ alone, for an overlay that keeps its own body (the inspect overlay's card and
//   its sheets, hero inspect, the pack scene): the one close glyph every dialog shows.

import {
  useId,
  useRef,
  type MouseEventHandler,
  type PointerEventHandler,
  type ReactElement,
  type ReactNode,
  type RefObject,
} from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import "./dialog.css";

export const dialogTestid = { close: "dialog-close" } as const;

/** One answer in the row, already translated. */
export type DialogAction = {
  label: string;
  /** Its button role (buttons.css); a `danger` answer goes last. */
  kind: "primary" | "secondary" | "danger";
  onClick(): void;
  testid: string;
  /** It waits, drawn but not pressable (a send that has nothing chosen yet). */
  disabled?: boolean;
  /** Its request is out: marked `aria-busy`. */
  busy?: boolean;
};

export type DialogProps = {
  /** The `h2`, already translated: a string, or a name drawn with its own markup. */
  title: ReactNode;
  /** The line under it, already translated: the dialog's description. */
  description?: string;
  /** Escape, Back, a click on the scrim, and the ✕ or the Cancel. */
  onClose(): void;
  /** The answers after the Cancel, safe first: the row moves a `danger` one last. */
  actions: readonly DialogAction[];
  testid: string;
  /** The panel's `id`, for an opener's `aria-controls`. */
  id?: string;
  /** The ✕'s testid, when the caller's tests know it by its own. */
  closeTestid?: string;
  /** The ✕'s name, already translated; "Close" unless the caller says what it closes. */
  closeLabel?: string;
  /** The scrim's testid, when a caller's tests click it. */
  scrimTestid?: string;
  /** R3221000: a sheet, at the foot of an upright phone and down the right edge of one on its side. */
  sheet?: boolean;
  /** What takes the focus on open; otherwise the first answer, otherwise the ✕. */
  focus?: RefObject<HTMLElement | null>;
  /** The panel, for a caller that reads it (its own focus on open). */
  panelRef?: RefObject<HTMLElement | null>;
  /** A modifier of the panel's look; it restyles the panel only. */
  className?: string;
  /** `data-*` attributes on the panel. */
  data?: Readonly<Record<`data-${string}`, string>>;
  /** Drawn above the title (the practice result's crest). */
  header?: ReactNode;
  /** Drawn between the line and the row. */
  children?: ReactNode;
  /** Drawn under the row. */
  footer?: ReactNode;
} & (
  | {
      role: "alertdialog";
      /** The safe choice, first in the row: it closes the dialog. */
      cancel: { label: string; kind: "primary" | "secondary"; testid: string; disabled?: boolean };
    }
  | { role: "dialog"; cancel?: never }
);

/** The ✕ every dialog closes by, at its panel's top right (dialog.css), named "Close" unless told. */
export function DialogClose({
  onClick,
  onPointerDown,
  testid = dialogTestid.close,
  label,
  disabled,
  buttonRef,
}: {
  onClick: MouseEventHandler<HTMLButtonElement>;
  onPointerDown?: PointerEventHandler<HTMLButtonElement>;
  testid?: string;
  label?: string;
  disabled?: boolean;
  buttonRef?: RefObject<HTMLButtonElement | null>;
}): ReactElement {
  useLocale();
  return (
    <button
      ref={buttonRef}
      type="button"
      className="dialog__close button-secondary"
      data-testid={testid}
      aria-label={label ?? t("Close")}
      disabled={disabled}
      onPointerDown={onPointerDown}
      onClick={onClick}
    >
      <span aria-hidden="true">✕</span>
    </button>
  );
}

export function Dialog(props: DialogProps): ReactElement {
  const { title, description, onClose, actions, testid, className, data, header, children, footer } = props;
  useLocale();
  const titleId = useId();
  const bodyId = useId();
  const ownRoot = useRef<HTMLElement>(null);
  const root = props.panelRef ?? ownRoot;
  const first = useRef<HTMLButtonElement>(null);
  const pressedInside = useRef(false);
  useModalKeys(root, onClose, props.focus ?? first);

  const row: readonly DialogAction[] = [
    ...(props.role === "alertdialog" ? [{ ...props.cancel, onClick: onClose }] : []),
    ...actions.filter((action) => action.kind !== "danger"),
    ...actions.filter((action) => action.kind === "danger"),
  ];

  return (
    <div
      className="dialog-scrim"
      data-testid={props.scrimTestid}
      {...(props.sheet === true ? { "data-sheet": "" } : {})}
      // A press that began in the panel and was let go over the scrim (a text selection dragged out of
      // the chat or an input) clicks the scrim, their nearest common ancestor: that one keeps it open.
      onPointerDownCapture={(event) => {
        pressedInside.current = event.target !== event.currentTarget;
      }}
      onClick={(event) => {
        const inside = pressedInside.current;
        pressedInside.current = false;
        if (event.target === event.currentTarget && !inside) onClose();
      }}
    >
      <section
        ref={root}
        id={props.id}
        className={className === undefined ? "dialog" : `dialog ${className}`}
        data-testid={testid}
        {...(data ?? {})}
        role={props.role}
        aria-modal="true"
        aria-labelledby={titleId}
        {...(description === undefined ? {} : { "aria-describedby": bodyId })}
        // A click on the panel's text keeps the focus in the dialog.
        tabIndex={-1}
      >
        {header}
        <h2 className="dialog__title" id={titleId}>
          {title}
        </h2>
        {description === undefined ? null : (
          <p className="dialog__body" id={bodyId}>
            {description}
          </p>
        )}
        {children}
        {row.length === 0 ? null : (
          <div className="dialog__actions" data-count={row.length}>
            {row.map((action, index) => (
              <button
                key={action.testid}
                ref={index === 0 ? first : undefined}
                type="button"
                className={`button-${action.kind}`}
                data-testid={action.testid}
                disabled={action.disabled}
                aria-busy={action.busy === true ? true : undefined}
                onClick={action.onClick}
              >
                {action.label}
              </button>
            ))}
          </div>
        )}
        {footer}
        {props.role === "dialog" ? (
          <DialogClose
            buttonRef={row.length === 0 ? first : undefined}
            testid={props.closeTestid}
            label={props.closeLabel}
            onClick={onClose}
          />
        ) : null}
      </section>
    </div>
  );
}
