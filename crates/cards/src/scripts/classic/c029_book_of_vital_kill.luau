--!strict
local J = require("@jackioh")

local VITAL_HEALTH = 13

-- C #16 Book of Flame, which the Radiant face adds on its base face.
local BOOK_OF_FLAME = "classic-016"

local function targets(): { J.TargetDecl }
	return {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "hero" } },
		},
	}
end

local function setVitalHealth(): J.EffectCall
	return J.set_health({ to = { of = "chosen" }, value = VITAL_HEALTH })
end

local base = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		return { setVitalHealth() }
	end,
}

local radiant = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		return {
			setVitalHealth(),
			J.add_to_hand({ defId = BOOK_OF_FLAME }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
