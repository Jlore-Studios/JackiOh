// Runs under the repo's tsx from its root and prints the source-of-truth lesson, tutorial, and Quickdraw data.
// Specs 22 and 23 compare their page state to it; runtime client imports are intentional (SPEC §9.10, R290, R291).

import { readFileSync } from "node:fs";

type Lesson = {
  id: string;
  number: number;
  title: string;
  seed: string;
  humanSeat: string;
  humanDeck: readonly string[];
  aiDeck: readonly string[];
};

type CatalogEntry = { id?: string; tags?: unknown };

async function main(): Promise<void> {
  const { TUTORIAL_LESSONS } = (await import("../../../apps/web/src/tutorial/lessons.ts")) as {
    TUTORIAL_LESSONS: readonly Lesson[];
  };
  const { AI_TUTORIAL } = (await import("../../../apps/web/src/wire/engineConfig.ts")) as {
    AI_TUTORIAL: Record<string, number>;
  };
  const catalog = JSON.parse(readFileSync("crates/cards/catalog.json", "utf8")) as Record<string, CatalogEntry>;
  const quickdraw = Object.entries(catalog)
    .filter(([, entry]) => Array.isArray(entry.tags) && entry.tags.includes("Quickdraw"))
    .map(([id]) => id);

  process.stdout.write(
    `${JSON.stringify({
      ok: true,
      lessons: TUTORIAL_LESSONS.map((lesson) => ({
        id: lesson.id,
        number: lesson.number,
        title: lesson.title,
        seed: lesson.seed,
        humanSeat: lesson.humanSeat,
        humanDeck: [...lesson.humanDeck],
        aiDeck: [...lesson.aiDeck],
      })),
      aiTutorial: { ...AI_TUTORIAL },
      quickdraw,
    })}\n`,
  );
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? `${error.message}\n${error.stack ?? ""}` : String(error);
  process.stdout.write(`${JSON.stringify({ ok: false, error: message })}\n`);
});
