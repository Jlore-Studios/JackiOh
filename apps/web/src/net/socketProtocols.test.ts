// R1379000: what a socket offers on its handshake, and the token read back from it as the server
// reads it.

import { describe, expect, it } from "vitest";

import { socketProtocols, tokenFromProtocols } from "./socketProtocols.ts";

describe("socketProtocols", () => {
  it("R1379000 offers jackioh.v1 first and the token as a base64url protocol name", () => {
    for (const token of ["e2e-token-p1", "a.b-c_d", "tök en/+="]) {
      const offered = socketProtocols(token);
      expect(offered).toHaveLength(2);
      expect(offered[0]).toBe("jackioh.v1");
      expect(offered[1]).toMatch(/^jackioh\.bearer\.[A-Za-z0-9_-]+$/u);
      expect(tokenFromProtocols(offered)).toBe(token);
    }
  });

  it("R1379000 encodes the token's UTF-8 as base64url without padding, as the server decodes it", () => {
    // The server's own vectors (`crates/server/tests/actor/ws_server.rs`): a `-` where standard
    // base64 has `+`, and no `=`.
    expect(socketProtocols("tök?>")[1]).toBe("jackioh.bearer.dMO2az8-");
    expect(socketProtocols("tök?")[1]).toBe("jackioh.bearer.dMO2az8");
  });

  it("R1379000 reads no token from a missing or undecodable credential", () => {
    for (const offered of [[], ["jackioh.v1"], ["jackioh.v1", "jackioh.bearer.!!!"], ["jackioh.v1", "jackioh.bearer."]]) {
      expect(tokenFromProtocols(offered)).toBeNull();
    }
  });
});
