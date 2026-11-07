// One dialog (#3220, R3220000): the board's confirms and the practice result draw through it, and every
// other dialog is to follow. It decides nothing (CLAUDE.md rule 7): each answer is its caller's.
//
// - A modal `dialog` or `alertdialog`, labelled by its `h2` title and described by its line when it has one.
// - Close: a `dialog` closes by its ✕ at the top right, named "Close"; an `alertdialog` has no ✕, its
//   Cancel (the safe choice) is the close.
// - Order: the safe choice first, holding the focus from the start, so a stray Enter or Space never
//   acts; the act last, at the right, and a `danger` answer always last. A third answer goes between,
//   and the safe choice then takes the first row (R765). The row stacks in the same order at 360 px.
// - Keys: `useModalKeys`'s. Escape, Back (a layer, R2127000) and a click on the scrim close it, Tab
//   stays inside, and the focus goes back to the opener unless the caller has moved it. It takes no
//   key handler of its own (R2126000).
// - Layer: `--z-dialog` (surfaces.css), over the prompt's scrim, the result overlay, the drag layer and
//   the toasts: it is the one question on the screen while it is open.

import { useId, useRef, type ReactElement, type ReactNode } from "react";

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
};

export type DialogProps = {
  /** The `h2`, already translated. */
  title: string;
  /** The line under it, already translated: the dialog's description. */
  description?: string;
  /** Escape, Back, a click on the scrim, and the ✕ or the Cancel. */
  onClose(): void;
  /** The answers after the Cancel, safe first: the row moves a `danger` one last. */
  actions: readonly DialogAction[];
  testid: string;
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
      cancel: { label: string; kind: "primary" | "secondary"; testid: string };
    }
  | { role: "dialog"; cancel?: never }
);

export function Dialog(props: DialogProps): ReactElement {
  const { title, description, onClose, actions, testid, className, data, header, children, footer } = props;
  useLocale();
  const titleId = useId();
  const bodyId = useId();
  const root = useRef<HTMLElement>(null);
  const first = useRef<HTMLButtonElement>(null);
  useModalKeys(root, onClose, first);

  const row: readonly DialogAction[] = [
    ...(props.role === "alertdialog" ? [{ ...props.cancel, onClick: onClose }] : []),
    ...actions.filter((action) => action.kind !== "danger"),
    ...actions.filter((action) => action.kind === "danger"),
  ];

  return (
    <div
      className="dialog-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        ref={root}
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
                onClick={action.onClick}
              >
                {action.label}
              </button>
            ))}
          </div>
        )}
        {footer}
        {props.role === "dialog" ? (
          <button
            ref={row.length === 0 ? first : undefined}
            type="button"
            className="dialog__close button-secondary"
            data-testid={dialogTestid.close}
            aria-label={t("Close")}
            onClick={onClose}
          >
            <span aria-hidden="true">✕</span>
          </button>
        ) : null}
      </section>
    </div>
  );
}
