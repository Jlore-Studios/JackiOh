// The mix bus (audio/mix.ts): bus levels for a settings value, the limiter B57 measured, and
// R669's seeded reverb impulse. A mutant moving a level, a threshold or the decay fails here.

import { describe, expect, it } from "vitest";

import { DEFAULT_AUDIO_SETTINGS } from "./settings.ts";
import { fakeContextFactory } from "./test/fakeAudio.ts";
import { buildMix, LIMITER, mixLevels, reverbImpulse } from "./mix.ts";

describe("audio mix", () => {
  it("holds B57's limiter at -6 dB with a 3 ms attack", () => {
    expect(LIMITER).toEqual({ thresholdDb: -6, kneeDb: 0, ratio: 20, attackS: 0.003, releaseS: 0.25 });
  });

  it("mixLevels mutes master and the voice bus, and passes the rest through", () => {
    expect(mixLevels(DEFAULT_AUDIO_SETTINGS)).toEqual({
      master: DEFAULT_AUDIO_SETTINGS.master,
      sfx: DEFAULT_AUDIO_SETTINGS.sfx,
      crowd: DEFAULT_AUDIO_SETTINGS.crowd,
      voice: DEFAULT_AUDIO_SETTINGS.voice,
      music: DEFAULT_AUDIO_SETTINGS.music,
    });
    expect(mixLevels({ ...DEFAULT_AUDIO_SETTINGS, muted: true }).master).toBe(0);
    expect(mixLevels({ ...DEFAULT_AUDIO_SETTINGS, voiceOn: false }).voice).toBe(0);
    expect(mixLevels({ ...DEFAULT_AUDIO_SETTINGS, muted: true }).sfx).toBe(DEFAULT_AUDIO_SETTINGS.sfx);
  });

  it("buildMix writes the levels onto the buses and the limiter", () => {
    const factory = fakeContextFactory();
    const ctx = factory.create();
    const settings = { ...DEFAULT_AUDIO_SETTINGS, master: 0.5, sfx: 0.3, crowd: 0.2, voice: 0.7, music: 0.4 };
    const mix = buildMix(ctx, settings);

    expect(mix.master.gain.value).toBe(0.5);
    expect(mix.sfx.gain.value).toBe(0.3);
    expect(mix.crowd.gain.value).toBe(0.2);
    expect(mix.voice.gain.value).toBe(0.7);
    expect(mix.music.gain.value).toBe(0.4);
    expect(mix.limiter.threshold.value).toBe(LIMITER.thresholdDb);
    expect(mix.limiter.ratio.value).toBe(LIMITER.ratio);
    expect(mix.limiter.attack.value).toBe(LIMITER.attackS);
    expect(mix.limiter.release.value).toBe(LIMITER.releaseS);
  });

  it("buildMix silences master and voice when the settings say so", () => {
    const factory = fakeContextFactory();
    const ctx = factory.create();
    const mix = buildMix(ctx, { ...DEFAULT_AUDIO_SETTINGS, muted: true, voiceOn: false });

    expect(mix.master.gain.value).toBe(0);
    expect(mix.voice.gain.value).toBe(0);
    expect(mix.sfx.gain.value).toBe(DEFAULT_AUDIO_SETTINGS.sfx);
  });

  it("reverbImpulse is the same seeded noise on every run, decaying and bounded", () => {
    const factory = fakeContextFactory();
    const first = reverbImpulse(factory.create());
    const second = reverbImpulse(factory.create());

    expect(first.length).toBe(Math.round(1.1 * first.sampleRate));
    expect(first.numberOfChannels).toBe(2);
    expect([...first.getChannelData(0)]).toEqual([...second.getChannelData(0)]);

    const data = first.getChannelData(0);
    let early = 0;
    let late = 0;
    for (let i = 0; i < data.length / 2; i += 1) early += Math.abs(data[i] ?? 0);
    for (let i = Math.floor(data.length / 2); i < data.length; i += 1) late += Math.abs(data[i] ?? 0);
    expect(early).toBeGreaterThan(0);
    expect(late).toBeLessThan(early);
    for (const value of data) expect(Math.abs(value)).toBeLessThanOrEqual(1);
  });
});
