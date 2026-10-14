--!strict
local J = require("@jackioh")

-- §7's shared Rush Token, 3/3 with Rush. The aura and the summon must name the same def.
local RUSH_TOKEN = "core-t-rush"

-- R62, §6.2: the controller's start of turn; R64 places the token in the leftmost empty, unlocked,
-- unreserved unit zone, and a full board summons nothing.
local function start_of_turn(ctx: J.EffectContext)
	return { J.summon({ defId = RUSH_TOKEN }) }
end

-- §10.4 layer 5, keyed on the def id: the Rush Tokens this card's controller controls (control is
-- what "your" means, R12), wherever the layers read one. A named `aura_filter` is handed the
-- candidate alone, so the hook names the one for its own controller.
local YOURS = { p1 = "aura_filter", p2 = "aura_filter_2" }

return {
	base = { startOfTurn = start_of_turn },
	-- §8 Conventions: "same" says so explicitly, so the radiant face adds the aura and keeps the summon.
	radiant = {
		startOfTurn = start_of_turn,
		aura = function(ctx: J.HookArgs)
			-- "+3/+3": the declared number `buff` (R386).
			local buff = ctx:param("buff")
			return { J.aura({ mod = { attack = buff, maxHealth = buff }, applies = YOURS[ctx:controller()] }) }
		end,
		aura_filter = function(unit: any): boolean
			return unit.controller == "p1" and unit.defId == RUSH_TOKEN
		end,
		aura_filter_2 = function(unit: any): boolean
			return unit.controller == "p2" and unit.defId == RUSH_TOKEN
		end,
	},
}
