--!strict
local J = require("@jackioh")

-- §8.3: the base text names 1-cost traps; §5.1's `cost` reads a def's cost out of play (R65).
local TRAP_COST = 1

-- The faces differ in the pool and in whether the trap comes Radiant, so one factory takes both.
-- The single read is the engine's own `slot_of`, which turns the Cry's unit into its zone.
local function oomen(pool: J.CatalogQueryArgs, radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			-- A reader's absent answer crosses as the host's null, a userdata rather than `nil`, so
			-- each test below asks for a table.
			-- The live `ctx.self`: its zone is what `slot_of` reads.
			local self_ = ctx:live_self()
			if type(self_) ~= "table" then
				return {}
			end
			-- Off the field there is no "this lane" to summon into, so nothing happens (§3.1).
			local at = ctx:slot_of(self_.id)
			if type(at) ~= "table" then
				return {}
			end
			return {
				J.summon_random({
					query = pool,
					player = "self",
					lane = at.lane,
					radiant = radiant,
				}),
			}
		end,
	}
end

-- Both pools: the Cost (1) traps, and every trap. `J.TRAP_TYPES` is typed `{string}`, not
-- `{CardType}`, so it is cast.
return {
	base = oomen({ type = J.TRAP_TYPES :: any, cost = TRAP_COST }, false),
	radiant = oomen({ type = J.TRAP_TYPES :: any }, true),
}
