--!strict
local J = require("@jackioh")

-- `add_random_from_catalog` with the `chinese` rider (ME-CN), set before the card moves as the
-- Radiant rider is. Both faces add the same count at the same discount; the Radiant face's cards
-- are Radiant.
local function first_day(radiant: boolean)
	-- Left off on the base face, as the Rust left the key off.
	local made_radiant: boolean? = if radiant then true else nil
	return {
		cry = function(ctx: J.EffectContext)
			local cards = ctx:param("cards")
			local discount = ctx:param("discount")
			return {
				J.add_random_from_catalog({
					query = { tags = { "CN" } },
					count = cards,
					costMod = -discount,
					chinese = true,
					radiant = made_radiant,
				}),
			}
		end,
	}
end

return {
	base = first_day(false),
	radiant = first_day(true),
}
