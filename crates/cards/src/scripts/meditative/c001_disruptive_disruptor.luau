--!strict
local J = require("@jackioh")

-- "You can't be forced to discard cards during your opponent's turn": the controller's own side,
-- guarded on every turn but their own (`draw::discard_guarded`).
local function guard(ctx: J.HookArgs)
	return { "self" }
end

local base = {
	discardGuard = guard,
	-- "Cry: Draw {draw}" (patch v0.3.9, #831): the declared number `draw` (R386).
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

local radiant = {
	discardGuard = guard,
	-- "Cry: Draw {draw} and gain {mana} mana": the declared numbers `draw` and `mana` (R386).
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
			J.gain_mana({ amount = ctx:param("mana") }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
