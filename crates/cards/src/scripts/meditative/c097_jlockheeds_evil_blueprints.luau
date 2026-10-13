--!strict
local J = require("@jackioh")

-- The nine buildings of the blueprint (M #97.1 to M #97.9).
local BUILDINGS = {
	"meditative-097-1",
	"meditative-097-2",
	"meditative-097-3",
	"meditative-097-4",
	"meditative-097-5",
	"meditative-097-6",
	"meditative-097-7",
	"meditative-097-8",
	"meditative-097-9",
}

-- §6.3 Discover offers three different options.
local OPTIONS = 3

-- The one resume step: the Discover's answer comes back here (§10.6, `prompts::RESUME_HOOK`).
local PICKED = "picked"

local function blueprints(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.discover_from_catalog({
					step = PICKED,
					count = OPTIONS,
					query = { defId = BUILDINGS, withTokens = true },
					prompt = "Jlockheed's Evil Blueprints: Discover a building",
				}),
			}
		end,
		resume = {
			[PICKED] = function(ctx: J.EffectContext)
				local picked = ctx:chosen_options()[1]
				if picked == nil then
					return {}
				end
				return {
					J.add_to_hand({ defId = picked, radiant = radiant }),
				}
			end,
		},
	}
end

return {
	base = blueprints(false),
	radiant = blueprints(true),
}
