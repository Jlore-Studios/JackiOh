// C #48 Hired Shrimp (SPEC §8.6 row 48; §5 `loc`, §10.8; R46, R70, R81, R90, R177, R397). Unit 4/3 →
// 8/6, cost 2, Common.
//   Base:    "Cry: Destroy a permanent whose card takes more lines of code to implement than this one."
//   Radiant: "… Valid targets are highlighted."
//   Engine:  the Cry's declared target (R81) may be any permanent on either side; at resolution it is
//            destroyed only if its `loc` is greater than Hired Shrimp's, else the Cry fizzles (R397).
//            Radiant: only permanents whose `loc` is greater are offered, except a face-down card the
//            chooser may not read, always offered and judged at resolution (R177). A fused card's
//            `loc` is its ingredients' sum; Indestructible stays.

import { defOf, type CardInstance, type GameState, type Script } from "@jackioh/engine";
import { destroy, instanceOf, unreadableBy } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-048");

const locOf = (state: GameState, defId: string): number => defOf(state, defId).loc ?? 0;

const longer = (state: GameState, card: CardInstance, shrimpDefId: string): boolean =>
  locOf(state, card.defId) > locOf(state, shrimpDefId);

function shrimp(check?: "longer"): Script {
  return {
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], ...(check === undefined ? {} : { check }) } }],
    targetChecks: {
      longer: ({ state, self, player, candidate }) =>
        candidate !== null && (unreadableBy(state, candidate).includes(player) || longer(state, candidate, self.defId)),
    },
    cry: (ctx) => {
      const target = instanceOf(ctx, { of: "chosen" });
      return target !== null && longer(ctx.state, target, ctx.self?.defId ?? def.id) ? [destroy({ target: { of: "chosen" } })] : [];
    },
  };
}

export const base: Script = shrimp();

export const radiant: Script = shrimp("longer");
