--!strict
local J = require("@jackioh")

-- "Fill your hand": this player's hand cap (R1143) less the cards in hand as it resolves. A full
-- hand adds nothing and draws nothing from the rng (R129).
local function bulk_booster(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local me = ctx:controller()
			local room = ctx:hand_cap_of(me) - ctx:zone_count(me, "hand")
			if room <= 0 then
				return {}
			end
			return {
				J.add_random_from_catalog({
					query = { rarity = "Common" },
					count = room,
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = bulk_booster(false),
	radiant = bulk_booster(true),
}
