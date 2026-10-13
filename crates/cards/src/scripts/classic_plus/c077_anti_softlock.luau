--!strict
local J = require("@jackioh")

local ALL = "All cards"
local YOURS = "Only yours"

-- The grants in the order written, so the drawn card gains the keywords too.
local function anti_softlock(ctx: J.EffectContext, side: J.ScopeSide): { J.EffectCall }
	local scope: J.CardScope = { side = side, zones = { "field", "hand", "library" } }
	return {
		J.draw({ count = ctx:param("draw") }),
		J.grant_keyword_cards({ scope = scope, keyword = { kind = "Stack" } }),
		J.grant_keyword_cards({ scope = scope, keyword = { kind = "Pierce" } }),
		J.unlock_all({}),
	}
end

local base = {
	cry = function(ctx: J.EffectContext)
		return anti_softlock(ctx, "any")
	end,
}

local radiant = {
	modes = {
		{ kind = "mode", options = { ALL, YOURS } },
	},
	cry = function(ctx: J.EffectContext)
		local side: J.ScopeSide = "any"
		if ctx:chosen_options()[1] == YOURS then
			side = "self"
		end
		return anti_softlock(ctx, side)
	end,
}

return {
	base = base,
	radiant = radiant,
}
