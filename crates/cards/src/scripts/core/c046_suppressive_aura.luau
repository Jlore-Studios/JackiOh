--!strict
local J = require("@jackioh")

-- The penalty at the price this card was played for (R65): the declared number `debuff` at the base
-- price and `paidDebuff` at the embiggen price (R386) — 1 and 2, 2 and 4 on the Radiant face. R78:
-- leaving the field resets `embiggened`.
local function penalty(ctx: J.HookArgs): number
	if ctx:self_().embiggened == true then
		return ctx:param("paidDebuff")
	end
	return ctx:param("debuff")
end

-- `-penalty` on both stats: §10.4 subtracts it from attack (floored at 0 on read) and from max
-- health (not floored, R69).
local function suppression(ctx: J.HookArgs): J.StatMod
	local n = penalty(ctx)
	return { attack = -n, maxHealth = -n }
end

-- "Enemy units": enemy of this card's controller, which is control and not ownership (R12), so a
-- stolen or rotated Suppressive Aura suppresses the other board from its new side. Neither face's
-- clause names a zone, so neither does its predicate: a named `aura_filter` is handed the candidate
-- alone, so the hook names the one that turns its own controller's units away.
local NOT_OF = { p1 = "aura_filter", p2 = "aura_filter_2" }

return {
	-- "All units": every unit on either side, the controller's own included.
	base = {
		aura = function(ctx: J.HookArgs)
			return { J.aura({ mod = suppression(ctx), applies = "aura_filter" }) }
		end,
		aura_filter = function(_unit: any): boolean
			return true
		end,
	},
	radiant = {
		aura = function(ctx: J.HookArgs)
			return { J.aura({ mod = suppression(ctx), applies = NOT_OF[ctx:controller()] }) }
		end,
		aura_filter = function(unit: any): boolean
			return unit.controller ~= "p1"
		end,
		aura_filter_2 = function(unit: any): boolean
			return unit.controller ~= "p2"
		end,
	},
}
