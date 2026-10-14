--!strict
local J = require("@jackioh")

local ID = "classicplus-052"

-- §10.6's data is JSON: a number carried in the entry, or the printed value when it is missing.
local function carried(ctx: J.EffectContext, key: string): number
	local value = ctx:data()[key]
	if type(value) == "number" then
		return value
	end
	return ctx:param(key)
end

-- `forRestOfGame` (B5 E28, R458) re-enters `delayed` at each of the caster's turn starts with no `self`
-- (R127), so the numbers are read through `param` as the Spell resolves and carried in the entry's data
-- (R594). The pool leaves this card out by its def id, which the re-entry carries (R387).
local function contract(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local cards = ctx:param("cards")
			local discount = ctx:param("discount")
			local face: J.FaceKind = if radiant then "radiant" else "base"
			local label = ctx:fill_params(ID, face, { cards = cards, discount = discount })
			return {
				J.for_rest_of_game({
					step = "contract",
					label = label,
					data = { cards = cards, discount = discount },
				}),
			}
		end,
		delayed = function(ctx: J.EffectContext)
			if radiant then
				return {
					J.add_random_from_catalog({
						query = { tags = { "Jlockeed" } },
						count = carried(ctx, "cards"),
						radiant = true,
						costMod = -carried(ctx, "discount"),
					}),
				}
			end
			return {
				J.add_random_from_catalog({
					query = { tags = { "Jlockeed" } },
					count = carried(ctx, "cards"),
				}),
			}
		end,
	}
end

return {
	base = contract(false),
	radiant = contract(true),
}
