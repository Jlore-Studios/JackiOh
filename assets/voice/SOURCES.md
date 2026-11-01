# Voice line sources

Where every spoken card line the client plays came from (issue #2631, R2631000), beside the music's
`assets/music/LICENSES.md`. `/ai` says the lines are computer text-to-speech; this is the record
behind that sentence.

## How the lines are made

- **The words** are each card's voice line in `crates/cards/card-text.yaml`, exported to
  `apps/web/src/audio/voice-text.json` (`cargo test -p jackioh-tools --test export_card_text`).
- **The voices** are named in the voices bank of `apps/web/src/audio/card-audio.json5`, each with its
  synthesizer, rate and pitch.
- **The rendering** is `apps/web/scripts/gen-voice.mjs` (`pnpm --filter @jackioh/web gen:voice`, or
  the `voice` workflow on GitHub's macOS and Windows runners for a commit holding `[gen:voice]`). It
  speaks each line with one of two operating-system text-to-speech engines and encodes it to AAC in
  `apps/web/public/audio/voice/<card id>-<hook>.m4a`; `voice-manifest.json` records each file's hash
  of its voice and text, and its size.
  - **macOS `say`**, encoded by `afconvert`, with Apple's system voices: Albert, Bahh, Boing, Bubbles,
    Daniel, Fred, Junior, Karen, Kathy, Moira, Ralph, Samantha, Superstar, Trinoids, Whisper, Wobble
    and Zarvox.
  - **Windows SAPI** (`System.Speech`), then trimmed, pitch-shifted and filtered by `ffmpeg`, with
    Microsoft's desktop voices: Microsoft David Desktop and Microsoft Zira Desktop.

No line is a recording of a person, and no voice was cloned from one: each is a stock voice that
ships with the operating system.

## Licence: the owner's to confirm

Unlike the music, these files use voices that belong to Apple and Microsoft, and each operating
system's licence sets what may be done with what its voices say. Apple's macOS licence, as we read it,
allows its system voices for personal, non-commercial use; publishing their output in a game on a
public website may need Apple's permission or a different voice. Microsoft's terms for the Windows
desktop voices need the same reading. Until the owner (and the lawyer in `docs/compliance.md`)
confirms both, treat these files as not cleared for commercial use. If they are not, re-render every
line with a voice licensed for distribution: change the voices bank and run `gen:voice`, and the
manifest and the asset tests follow.
