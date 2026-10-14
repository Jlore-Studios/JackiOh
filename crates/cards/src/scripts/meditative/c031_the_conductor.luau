--!strict
local J = require("@jackioh")

-- One named draw per Created card in the controller's library, top down (MD-B7, R944). The list is
-- read once, as `for_each_card` reaches it, so an id gone by its turn fizzles and a card that enters
-- during the draws is not drawn.
local function created(ctx: J.EffectContext): { string }
	return ctx:created_in_library(ctx:controller())
end

local function draw_each(id: string)
	return { J.draw_from_library({ instanceId = id }) }
end

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
		}
	end,
	each_cards = created,
	each_card = draw_each,
}

-- First a random Prime card shuffled in (every Prime card is a token, R1421): it is Created, so the
-- same draws take it.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.shuffle_random_from_catalog({ query = { tags = { "Prime" } }, count = 1 }),
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
		}
	end,
	each_cards = created,
	each_card = draw_each,
}

return {
	base = base,
	radiant = radiant,
}
