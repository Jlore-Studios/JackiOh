--!strict
local J = require("@jackioh")

-- "+1/+1" (Radiant "+2/+2") for each AI generated card played.
local PER_AI_CARD = 1
local RADIANT_PER_AI_CARD = 2

local base = {
	-- §10.4 layer 2 over its controller's per-game count of AI-tagged plays (B5 E4).
	setStat = function(ctx: J.HookArgs)
		local per = if ctx:radiant() then RADIANT_PER_AI_CARD else PER_AI_CARD
		local bonus = per * ctx:played_this_game_with_tag(ctx:controller(), "AI")
		return { attack = bonus, maxHealth = bonus }
	end,
}

-- The same script: the hook reads the running face.
return {
	base = base,
	radiant = "base",
}
