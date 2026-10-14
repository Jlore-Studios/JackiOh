--!strict
local J = require("@jackioh")

-- TS `cardDef("core-t-rush").id`.
local RUSH_TOKEN = "core-t-rush"

local function apple(radiant: boolean)
	return {
		-- §8 Conventions: "Deal N damage" with no target named is targeted — any unit or hero, either side.
		targets = {
			{
				kind = "target",
				min = 1,
				max = 1,
				filter = { side = "any", of = { "unit", "hero" } },
			},
		},
		cry = function(ctx: J.EffectContext)
			local hero: J.TargetSpec = { of = "selfHero" }
			return {
				J.summon({ defId = RUSH_TOKEN, radiant = radiant }),
				J.heal({ target = hero, amount = ctx:param("heal") }),
				J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
			}
		end,
	}
end

return {
	base = apple(false),
	radiant = apple(true),
}
