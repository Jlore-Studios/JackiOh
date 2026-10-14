--!strict
local J = require("@jackioh")

-- The `data`-less resume steps: the cost answer opens the editor, the craft answer mints.
local COST = "cost"
local CRAFT = "craft"

-- The two faces differ only in `radiant`, as Core #99's differ only in their Discover count. The
-- crafted card's cost runs from 0 to `CRAFT_MAX_COST` (CLAUDE.md rule 9).
local function true_craft_a_card(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.choose_number({
					step = COST,
					from = 0,
					to = J.config.CRAFT_MAX_COST,
					prompt = "Choose the crafted card's Cost",
				}),
			}
		end,
		resume = {
			[COST] = function(ctx: J.EffectContext)
				local cost = ctx:chosen_number()
				if cost == nil then
					return {}
				end
				return {
					J.choose_craft({
						step = CRAFT,
						cost = cost,
						radiant = radiant,
					}),
				}
			end,
			[CRAFT] = function(ctx: J.EffectContext)
				local recipe = ctx:chosen_recipe()
				if recipe == nil then
					return {}
				end
				return {
					J.craft_card({
						recipe = recipe,
						radiant = radiant,
					}),
				}
			end,
		},
	}
end

return {
	base = true_craft_a_card(false),
	radiant = true_craft_a_card(true),
}
