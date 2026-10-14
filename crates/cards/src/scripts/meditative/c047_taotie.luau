--!strict
local J = require("@jackioh")

-- End of turn (R62): this is the kept target, and `fuse_cards` draws one ingredient from the match
-- rng (ME-FUSE-RANDOM, MD-C20): an enemy permanent on the field, then, on the Radiant face, a card
-- of the opponent's library. With no enemy permanent nothing is fused; an Immutable 饕餮 eats nothing
-- (R23).
local function taotie(radiant: boolean)
	return {
		endOfTurn = function(ctx: J.EffectContext)
			local me = ctx:live_self()
			if me == nil then
				return {}
			end
			local fusions = {
				J.fuse_cards({ targetInstanceId = me.id, randomIngredient = { side = "enemy", zones = { "field" } } }),
			}
			if radiant then
				fusions[#fusions + 1] = J.fuse_cards({
					targetInstanceId = me.id,
					randomIngredient = { side = "enemy", zones = { "library" } },
				})
			end
			return fusions
		end,
	}
end

return {
	base = taotie(false),
	radiant = taotie(true),
}
