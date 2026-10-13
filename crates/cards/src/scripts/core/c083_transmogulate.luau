--!strict
local J = require("@jackioh")

local ID = "core-083"

-- R35's "other zones", in the order this card walks them; the hand is one of them (R365).
local OFF_FIELD_ZONES: { J.OffFieldZone } = { "hand", "library", "graveyard", "exile" }

-- R35's pool: every non-token Legendary but this card (R387), of every set (R380), through §5.1's
-- one query (`J.pool`), never listed here. `type` narrows it to one card type, or to both trap types.
-- (`J.pool`'s typed stub takes one argument; the binding takes the excluded id and the query and
-- answers the definitions, hence the cast.)
local function legendaries(type_: any): { any }
	local query: { [string]: any } = { rarity = "Legendary" }
	if type_ ~= nil then
		query.type = type_
	end
	return (J.pool :: any)(ID, query)
end

-- R35 on the board: same type, with Field Trap counting as Trap in both directions.
local function same_type_legendaries(type_: string): { any }
	if type_ == "Trap" or type_ == "Field Trap" then
		return legendaries(J.TRAP_TYPES)
	end
	return legendaries(type_)
end

-- §3.2 and R13: "your board" is the card acting in each zone, units then backrow, lane by lane
-- (`permanents_on_field` walks the controller's side first). R35 and R23: an Immutable board card
-- stays, so it is left out here, and no pick is rolled for it (R129).
local function board_cards(ctx: J.EffectContext): { any }
	local me = ctx:controller()
	local cards = {}
	for _, card in ipairs(ctx:permanents_on_field(me)) do
		if card.zone.player == me and not ctx:unit_has(card.id, "Immutable") then
			cards[#cards + 1] = card
		end
	end
	return cards
end

-- One off-field pile of the controller's. R223: the library is walked in an order of the seed's own
-- (`numbering_order`), so no replacement's new id says where it lies; each still takes its card's place.
local function pile_cards(ctx: J.EffectContext, zone: J.OffFieldZone): { any }
	local cards = ctx:zone_cards(ctx:controller(), zone)
	if zone == "library" then
		return ctx:numbering_order(cards)
	end
	return cards
end

-- §6.3 Replace: one card, one random Legendary from its pool, named by instance (R81 does not apply).
local function replace(ctx: J.EffectContext, effects: { J.EffectCall }, card: any, pool: { any }, radiant: boolean)
	local pick = ctx:rng_pick(pool)
	if pick == nil then
		return
	end
	effects[#effects + 1] = J.transform({ instanceId = card.id, defId = pick.id, radiant = radiant })
end

-- The faces differ only in whether the cards that arrive are Radiant (§5.2, R74).
local function transmogulate(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {}
			-- The board first, each card by its own type (R35); lane order, so the rng draws are fixed.
			for _, card in ipairs(board_cards(ctx)) do
				local pool = same_type_legendaries(ctx:def_of(card.defId).type)
				replace(ctx, effects, card, pool, radiant)
			end
			-- Then hand (R365), library (in `pile_cards` order, R223), graveyard and exile: "any card
			-- from the pool, same counts". The pool is one query, the same for every card.
			local pool = legendaries(nil)
			for _, zone in ipairs(OFF_FIELD_ZONES) do
				for _, card in ipairs(pile_cards(ctx, zone)) do
					replace(ctx, effects, card, pool, radiant)
				end
			end
			return effects
		end,
	}
end

return {
	base = transmogulate(false),
	-- "Random Radiant Legendaries": the same five zones, every replacement Radiant.
	radiant = transmogulate(true),
}
