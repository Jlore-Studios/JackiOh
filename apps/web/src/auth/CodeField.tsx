// One code, typed into one field and drawn as groups (SPEC §9.4, R191). ONE INPUT, NOT FOUR keeps
// paste, autofill, undo and screen readers working; four `aria-hidden` segments draw what it holds,
// its text and highlight transparent (`color: transparent`, never `opacity: 0`: Cypress reads that as
// hidden). NOTHING IS DROPPED: every change is read as the server reads it, and a keystroke that adds
// a problem is refused and named, since dropping it would shift every later character. No `maxLength`:
// it would cut a padded paste before `onChange` saw it. A KEYBOARD THAT COMPOSES IS LEFT TO FINISH:
// rewriting a controlled value mid-IME-composition (Gboard) doubles or drops characters, so nothing is
// refused until the composition ends.

import {
  useLayoutEffect,
  useReducer,
  useRef,
  useState,
  type ChangeEvent,
  type ClipboardEvent,
  type CompositionEvent,
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent,
  type ReactElement,
} from "react";

import {
  findCodeInText,
  formattedCaret,
  isCodeSeparator,
  normalizeCodeText,
  readCodeInput,
  type CodeFormat,
  type CodeInputProblem,
} from "@jackioh/shared";

import {
  INVITE_CODE_FORMAT,
  codeProblemMessage,
  codeProgressText,
  groupCount,
  pastedText,
  placeholderFor,
} from "./codeInput.ts";
import { codeFieldSegmentTestid, codeFieldTestid, inviteTestid } from "./testids.ts";

import "./code-field.css";

export type CodeFieldProps = {
  /** The `<input>` id, for `<label htmlFor>`. */
  id: string;
  /** Formatted, controlled. */
  value: string;
  onChange: (next: { formatted: string; complete: boolean }) => void;
  /** Defaults to `INVITE_CODE_FORMAT`. */
  format?: CodeFormat;
  /** Defaults to `inviteTestid.input` (`invite-code-input`). */
  testId?: string;
  disabled?: boolean;
  /** Extra `aria-describedby` ids. */
  describedBy?: string;
};

function codeCharacterCount(text: string): number {
  return normalizeCodeText(text).length;
}

function charactersOf(formatted: string): string {
  let characters = "";
  for (const character of formatted) {
    if (!isCodeSeparator(character)) characters += character;
  }
  return characters;
}

function charactersBefore(formatted: string, position: number): number {
  return charactersOf(formatted.slice(0, position)).length;
}

/** ASCII letters and digits in `text`: how much code-like material a paste carries. */
function letterAndDigitCount(text: string): number {
  let count = 0;
  for (const character of text.normalize("NFKC")) {
    if (/^[0-9A-Za-z]$/u.test(character)) count += 1;
  }
  return count;
}

function clamp(value: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, value));
}

type SegmentState = "empty" | "partial" | "complete";

export default function CodeField(props: CodeFieldProps): ReactElement {
  const format = props.format ?? INVITE_CODE_FORMAT;
  const testId = props.testId ?? inviteTestid.input;
  const { id, value, onChange, disabled = false, describedBy } = props;

  const inputRef = useRef<HTMLInputElement>(null);
  const segmentsRef = useRef<HTMLDivElement>(null);
  /** The caret to restore once React has written the value, or null for "leave it alone". */
  const pendingCaret = useRef<number | null>(null);
  const [problem, setProblem] = useState<CodeInputProblem | null>(null);
  const [refusedText, setRefusedText] = useState<string | null>(null);
  const [focused, setFocused] = useState(false);
  const [caretCharacters, setCaretCharacters] = useState(0);
  const [selectedTo, setSelectedTo] = useState(0);
  // A change the parent turns into the value it already holds re-renders nothing upstream, so the
  // caret would never be put back. This does.
  const [, rerender] = useReducer((count: number) => count + 1, 0);
  /** What an IME composition has written so far, shown as it is (A KEYBOARD THAT COMPOSES). */
  const [draft, setDraft] = useState<string | null>(null);

  const reading = readCodeInput(value, format);
  const characters = reading.characters;
  const groups = groupCount(format);

  useLayoutEffect(() => {
    const input = inputRef.current;
    const caret = pendingCaret.current;
    if (input === null || caret === null) return;
    pendingCaret.current = null;
    const at = clamp(caret, 0, input.value.length);
    input.setSelectionRange(at, at);
    collapseAt(charactersBefore(input.value, at));
  });

  function collapseAt(count: number): void {
    setCaretCharacters(count);
    setSelectedTo(count);
  }

  function placeCaret(position: number): void {
    pendingCaret.current = position;
    rerender();
  }

  function commit(formatted: string, complete: boolean): void {
    if (formatted !== value) onChange({ formatted, complete });
  }

  function handleChange(event: ChangeEvent<HTMLInputElement>): void {
    if (disabled) return;
    const raw = event.target.value;
    if (draft !== null || (event.nativeEvent as { isComposing?: boolean }).isComposing === true) {
      whileComposing(raw);
      return;
    }
    readChange(raw, event.target.selectionStart ?? raw.length);
  }

  function whileComposing(raw: string): void {
    setDraft(raw);
    const next = readCodeInput(raw, format);
    setProblem(next.problem);
    setRefusedText(null);
    if (next.problem === null) commit(next.formatted, next.complete);
  }

  function handleCompositionStart(event: CompositionEvent<HTMLInputElement>): void {
    if (disabled) return;
    setDraft(event.currentTarget.value);
  }

  function handleCompositionEnd(event: CompositionEvent<HTMLInputElement>): void {
    if (disabled || draft === null) return;
    const input = event.currentTarget;
    const raw = input.value;
    setDraft(null);
    readChange(raw, input.selectionStart ?? raw.length);
  }

  function readChange(raw: string, rawCaret: number): void {
    const next = readCodeInput(raw, format);
    const inserted = raw.length - value.length;

    if (next.problem !== null && inserted > 1) {
      // A REFUSED PASTE STAYS IN VIEW. Text inserted at once (paste, drop, clipboard chip) is read for
      // a code first: the inserted text (it replaces what the field held), then the whole value.
      const insertedText = raw.slice(clamp(rawCaret - inserted, 0, raw.length), rawCaret);
      const found = findCodeInText(insertedText, format) ?? findCodeInText(raw, format);
      if (found !== null) {
        const reading = readCodeInput(found, format);
        setProblem(null);
        setRefusedText(null);
        placeCaret(reading.formatted.length);
        commit(reading.formatted, reading.complete);
        return;
      }

      // Else a paste at the end keeps its good part, unless that is already a whole code or the paste
      // holds more letters and digits than a code (a message): then the paste is quoted instead.
      const atEnd = rawCaret === raw.length;
      const keepsPrefix =
        atEnd &&
        next.problem.kind !== "tooLong" &&
        next.characters.length < format.length &&
        letterAndDigitCount(insertedText) <= format.length;
      setProblem(next.problem);
      if (keepsPrefix) {
        setRefusedText(null);
        placeCaret(next.formatted.length);
        commit(next.formatted, false);
        return;
      }
      setRefusedText(insertedText);
      placeCaret(clamp(rawCaret - inserted, 0, value.length));
      return;
    }

    if (next.problem !== null) {
      // Refused: the value stays, and so does the caret, where it was before the edit.
      setProblem(next.problem);
      setRefusedText(null);
      const inserted = Math.max(0, raw.length - value.length);
      placeCaret(clamp(rawCaret - inserted, 0, value.length));
      return;
    }

    setProblem(null);
    setRefusedText(null);
    const before = clamp(codeCharacterCount(raw.slice(0, rawCaret)), 0, next.characters.length);
    placeCaret(formattedCaret(before, format));
    commit(next.formatted, next.complete);
  }

  function handlePaste(event: ClipboardEvent<HTMLInputElement>): void {
    if (disabled) return;
    const found = findCodeInText(event.clipboardData.getData("text/plain"), format);
    if (found === null) return;
    event.preventDefault();
    const next = readCodeInput(found, format);
    setProblem(null);
    setRefusedText(null);
    placeCaret(next.formatted.length);
    commit(next.formatted, next.complete);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>): void {
    if (disabled) return;
    const input = event.currentTarget;
    const start = input.selectionStart;
    const end = input.selectionEnd;
    if (start === null || start !== end) return;
    const previous = start > 0 ? value[start - 1] : undefined;
    const following = value[start];
    const afterSeparator = previous !== undefined && isCodeSeparator(previous);
    const beforeSeparator = following !== undefined && isCodeSeparator(following);

    // Separators are drawn, not typed: deleting one would change nothing, and the next keystroke
    // would land on it again, so every one of these keys steps over it.
    if (event.key === "Backspace" && afterSeparator) {
      event.preventDefault();
      const before = charactersBefore(value, start);
      if (before === 0) return;
      const all = charactersOf(value);
      const next = readCodeInput(all.slice(0, before - 1) + all.slice(before), format);
      setProblem(null);
      setRefusedText(null);
      placeCaret(formattedCaret(before - 1, format));
      commit(next.formatted, next.complete);
      return;
    }
    if (event.key === "Delete" && beforeSeparator) {
      event.preventDefault();
      const before = charactersBefore(value, start);
      const all = charactersOf(value);
      if (before >= all.length) return;
      const next = readCodeInput(all.slice(0, before) + all.slice(before + 1), format);
      setProblem(null);
      setRefusedText(null);
      placeCaret(formattedCaret(before, format));
      commit(next.formatted, next.complete);
      return;
    }
    // Both sides of a separator draw in one place: an arrow that only crossed it would seem to do nothing.
    if (event.shiftKey || event.altKey || event.metaKey || event.ctrlKey) return;
    if (event.key === "ArrowRight" && beforeSeparator) {
      event.preventDefault();
      moveCaret(Math.min(start + 2, value.length));
    } else if (event.key === "ArrowLeft" && afterSeparator) {
      event.preventDefault();
      moveCaret(Math.max(start - 2, 0));
    }
  }

  function moveCaret(position: number): void {
    const input = inputRef.current;
    if (input === null) return;
    input.setSelectionRange(position, position);
    collapseAt(charactersBefore(value, position));
  }

  function syncCaret(): void {
    const input = inputRef.current;
    if (input === null) return;
    const start = input.selectionStart ?? input.value.length;
    const end = Math.max(start, input.selectionEnd ?? start);
    setCaretCharacters(charactersBefore(input.value, start));
    setSelectedTo(charactersBefore(input.value, end));
  }

  // The input's text is not laid out over the segments, so a click is mapped to the nearest cell.
  function handleMouseUp(event: MouseEvent<HTMLInputElement>): void {
    const root = segmentsRef.current;
    const input = inputRef.current;
    if (root === null || input === null) return;
    if (input.selectionStart !== input.selectionEnd) return; // a drag-selection is left alone
    const cells = Array.from(root.querySelectorAll<HTMLElement>("[data-cell]"));
    let nearest = -1;
    let distance = Number.POSITIVE_INFINITY;
    cells.forEach((cell, index) => {
      const rect = cell.getBoundingClientRect();
      if (rect.width <= 0) return; // no layout (jsdom): keep the native caret
      const gap = Math.abs(event.clientX - (rect.left + rect.width / 2));
      if (gap < distance) {
        distance = gap;
        nearest = index;
      }
    });
    if (nearest < 0) return;
    const at = formattedCaret(Math.min(nearest, characters.length), format);
    input.setSelectionRange(at, at);
    collapseAt(charactersBefore(value, at));
  }

  const activeSegment = focused
    ? Math.min(Math.floor(caretCharacters / format.groupSize), groups - 1)
    : -1;
  const selecting = focused && selectedTo > caretCharacters;

  const progressId = `${id}-progress`;
  const hintId = `${id}-hint`;
  const describedByIds = [progressId, problem === null ? null : hintId, describedBy ?? null]
    .filter((part): part is string => part !== null && part.length > 0)
    .join(" ");

  const segments: ReactElement[] = [];
  for (let index = 0; index < groups; index += 1) {
    const offset = index * format.groupSize;
    const size = Math.min(format.groupSize, format.length - offset);
    const filled = characters.slice(offset, offset + size);
    const state: SegmentState =
      filled.length === 0 ? "empty" : filled.length === size ? "complete" : "partial";
    const active = index === activeSegment;
    const caretInSegment = caretCharacters - offset;

    const cells: ReactElement[] = [];
    for (let slot = 0; slot < size; slot += 1) {
      const character = filled[slot];
      const at = offset + slot;
      const selected = selecting && character !== undefined && at >= caretCharacters && at < selectedTo;
      let caret: "before" | "after" | undefined;
      if (!selecting && active && caretInSegment === slot) caret = "before";
      else if (!selecting && active && slot === size - 1 && caretInSegment >= size) caret = "after";
      cells.push(
        <span
          key={slot}
          className="code-field__cell"
          data-cell=""
          data-filled={character === undefined ? "false" : "true"}
          {...(selected ? { "data-selected": "true" } : {})}
          {...(caret === undefined ? {} : { "data-caret": caret })}
        >
          {character ?? ""}
        </span>,
      );
    }

    segments.push(
      <span
        key={index}
        className="code-field__segment"
        data-testid={codeFieldSegmentTestid(index)}
        aria-hidden="true"
        data-state={state}
        data-active={active ? "true" : "false"}
      >
        {cells}
      </span>,
    );
  }

  return (
    <div
      className="code-field"
      data-testid={codeFieldTestid.root}
      data-complete={reading.complete ? "true" : "false"}
      {...(problem === null ? {} : { "data-problem": problem.kind })}
      data-disabled={disabled ? "true" : "false"}
      style={{ "--code-field-groups": String(groups) } as CSSProperties}
    >
      <div className="code-field__box">
        <div className="code-field__segments" ref={segmentsRef}>
          {segments}
        </div>
        <input
          ref={inputRef}
          id={id}
          className="code-field__input"
          data-testid={testId}
          name="code"
          type="text"
          inputMode="text"
          autoComplete="off"
          autoCapitalize="characters"
          autoCorrect="off"
          spellCheck={false}
          placeholder={placeholderFor(format)}
          value={draft ?? value}
          disabled={disabled}
          aria-invalid={problem === null ? "false" : "true"}
          aria-describedby={describedByIds}
          onChange={handleChange}
          onCompositionStart={handleCompositionStart}
          onCompositionEnd={handleCompositionEnd}
          onPaste={handlePaste}
          onKeyDown={handleKeyDown}
          onKeyUp={syncCaret}
          onSelect={syncCaret}
          onMouseUp={handleMouseUp}
          onFocus={() => {
            setFocused(true);
            syncCaret();
          }}
          onBlur={() => {
            setFocused(false);
          }}
        />
      </div>

      <p className="code-field__progress" id={progressId} data-testid={codeFieldTestid.progress} aria-live="polite">
        {codeProgressText(reading, format)}
      </p>

      {problem === null ? null : (
        <p
          className="code-field__hint"
          id={hintId}
          data-testid={codeFieldTestid.hint}
          role="status"
          data-kind={problem.kind}
        >
          {codeProblemMessage(problem, format)}
          {refusedText === null ? null : ` ${pastedText(refusedText, format)}`}
        </p>
      )}
    </div>
  );
}
