--!strict
local J = require("@jackioh")

-- "Cry: Draw 1": the declared number `draw` (R386), on both faces.
local function cry(ctx: J.EffectContext)
	return {
		J.draw({ count = ctx:param("draw") }),
	}
end

-- ME-ALTPLAY: "You may play your Spells face-down as Traps".
local base = {
	cry = cry,
	faceDownPlay = function(ctx: J.HookArgs)
		return {
			{ spells = true },
		}
	end,
}

-- The same permission, each Spell set this way gaining Echo +`echo` (R386).
local radiant = {
	cry = cry,
	faceDownPlay = function(ctx: J.HookArgs)
		return {
			{ spells = true, echo = ctx:param("echo") },
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
