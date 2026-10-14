--!strict
local J = require("@jackioh")

-- The replacement is data; its `then` step is owed on `state.work` and runs after the hit has
-- landed. `draw_count` is the face's: the declared number, or the deck as the step begins (R58).
local function after_redirect(ctx: J.EffectContext, draw_count: number): { J.EffectCall }
	-- R216: the re-aimed hit left the enemy hero at 0 or less, so the game is over. The record
	-- the follow-up was handed (`replacement_of`): an object that names its event.
	local record = ctx:data().replaced
	if type(record) == "table" and record.event ~= nil then
		local to = record.redirectedTo
		if to ~= nil and ctx:hero_of(to).health <= 0 then
			return {}
		end
	end
	local target: J.TargetSpec = { of = "selfHero" }
	return {
		J.heal({ target = target, amount = ctx:param("heal") }),
		J.draw({ count = draw_count }),
	}
end

local REPLACEMENTS = {
	{
		id = "final-gambit",
		on = "lethalHit",
		instead = { redirect = "enemyHero" },
		["then"] = "afterRedirect",
	},
}

local base = {
	replacements = REPLACEMENTS,
	resume = {
		afterRedirect = function(ctx: J.EffectContext)
			return after_redirect(ctx, ctx:param("draw"))
		end,
	},
}

-- "… and draw your deck": the deck as the step begins (R58).
local radiant = {
	replacements = REPLACEMENTS,
	resume = {
		afterRedirect = function(ctx: J.EffectContext)
			return after_redirect(ctx, ctx:zone_count(ctx:controller(), "library"))
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
