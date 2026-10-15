// The creator's code view (#920, K3): the card's kit script, editable, highlighted (`highlight.ts`),
// with each problem the Luau module's check found shown on its line. The text area is the editor;
// the highlighted copy is drawn under it, line for line, and never read.

import { useRef, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import type { ScriptProblem } from "../wasm/index.ts";
import { tokenLines } from "./highlight.ts";
import { creatorTestid } from "./testids.ts";

export type CodeViewProps = {
  source: string;
  problems: readonly ScriptProblem[];
  onChange: (source: string) => void;
};

export function CodeView({ source, problems, onChange }: CodeViewProps): ReactElement {
  const lines = tokenLines(source);
  const marked = new Map<number, string[]>();
  for (const problem of problems) {
    if (problem.line === undefined || problem.line === null) continue;
    marked.set(problem.line, [...(marked.get(problem.line) ?? []), problem.message]);
  }
  const under = useRef<HTMLPreElement>(null);
  return (
    <div className="creator-code" data-testid={creatorTestid.code}>
      <div className="creator-code__frame">
        <pre className="creator-code__lines" aria-hidden="true" ref={under}>
          {lines.map((tokens, index) => {
            const line = index + 1;
            const messages = marked.get(line);
            return (
              <div
                key={line}
                className={messages === undefined ? "creator-code__line" : "creator-code__line creator-code__line--problem"}
                data-line={line}
                {...(messages === undefined ? {} : { "data-testid": creatorTestid.codeProblemLine })}
              >
                <span className="creator-code__number">{line}</span>
                <span className="creator-code__text">
                  {tokens.map((token, at) => (
                    <span key={at} className={`creator-tok creator-tok--${token.kind}`}>
                      {token.text}
                    </span>
                  ))}
                  {"\n"}
                </span>
              </div>
            );
          })}
        </pre>
        <textarea
          className="creator-code__input"
          data-testid={creatorTestid.codeInput}
          aria-label={t("The card's script")}
          spellCheck={false}
          autoCapitalize="off"
          autoComplete="off"
          autoCorrect="off"
          wrap="off"
          value={source}
          onChange={(event) => onChange(event.target.value)}
          onScroll={(event) => {
            if (under.current === null) return;
            under.current.scrollTop = event.currentTarget.scrollTop;
            under.current.scrollLeft = event.currentTarget.scrollLeft;
          }}
        />
      </div>
    </div>
  );
}

/** The script's problems as a list, each with its line when it has one. */
export function ScriptProblems({ problems }: { problems: readonly ScriptProblem[] }): ReactElement | null {
  if (problems.length === 0) return null;
  return (
    <ul className="creator-problems" data-testid={creatorTestid.scriptProblems}>
      {problems.map((problem, index) => (
        <li key={index} className="creator-problems__item" data-testid={creatorTestid.scriptProblem}>
          {problem.line === undefined || problem.line === null ? (
            <span>{problem.message}</span>
          ) : (
            <>
              <strong>{t("Line {line}", { line: String(problem.line) })}</strong> <span>{problem.message}</span>
            </>
          )}
        </li>
      ))}
    </ul>
  );
}
