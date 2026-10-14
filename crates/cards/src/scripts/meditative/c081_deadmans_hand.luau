--!strict
local J = require("@jackioh")

-- The declared `chance` is a percentage.
local PERCENT = 100

-- One copy of each card in hand, in hand order, then of this card on the face it was played with
-- (R57's riders through `copyOf`). Paid at (2), each copy whose card is not already Radiant rolls
-- `chance` once, before it goes in (R1144, R311); at (0) none rolls, and a Radiant card's copy is
-- Radiant without a roll (R129). Then the Radiant face's draw.
local function deadmans_hand(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local chance = ctx:param("chance")
			local sources = {}
			for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
				sources[#sources + 1] = { id = card.id, defId = card.defId, radiant = card.radiant == true }
			end
			local this = ctx:self_()
			if this ~= nil then
				sources[#sources + 1] = { id = this.id, defId = this.defId, radiant = ctx:radiant() }
			end
			local effects = {}
			for _, source in ipairs(sources) do
				local radiant_copy = source.radiant or (ctx:embiggened() and ctx:rng_chance(chance, PERCENT))
				effects[#effects + 1] = J.shuffle_into({
					defId = source.defId,
					count = 1,
					radiant = radiant_copy,
					copyOf = source.id,
				})
			end
			if radiant then
				effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
			end
			return effects
		end,
	}
end

return {
	base = deadmans_hand(false),
	radiant = deadmans_hand(true),
}
