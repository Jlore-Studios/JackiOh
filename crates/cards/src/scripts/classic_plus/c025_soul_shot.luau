--!strict
local J = require("@jackioh")

-- A card on no lane sorts after every lane (TS `?? Number.MAX_SAFE_INTEGER`, Rust `i32::MAX`).
local NO_LANE = 2147483647

-- R414: higher attack plus current health, then higher cost, then lower lane.
local function better(ctx: J.EffectContext, a: any, b: any): any
	local function worth(card: any): number
		local view = ctx:unit_view(card.id)
		return view.attack + view.health
	end
	local function lane(card: any): number
		local at = ctx:slot_of(card.id)
		if type(at) == "table" then
			return at.lane
		end
		return NO_LANE
	end
	if worth(a) ~= worth(b) then
		return if worth(a) > worth(b) then a else b
	end
	if ctx:cost_now(a.id) ~= ctx:cost_now(b.id) then
		return if ctx:cost_now(a.id) > ctx:cost_now(b.id) then a else b
	end
	return if lane(b) < lane(a) then b else a
end

-- One random enemy Unit a Spell may affect, Lucky X picks keeping the best (the board is not empty).
local function pick(ctx: J.EffectContext): { string }
	local pool = ctx:cards_in_scope({ side = "enemy" })
	-- R987: the controller's Luck rolls extra times beside the card's own Lucky.
	local lucky = 0
	local me = ctx:live_self()
	if type(me) == "table" then
		for _, keyword in ipairs(ctx:card_keywords(me.id)) do
			if keyword.kind == "Lucky" then
				lucky += keyword.n
			end
		end
	end
	lucky += ctx:luck_of(ctx:controller())
	-- `Rng::lucky`: one roll, then X more, each kept against the best so far.
	local chosen = ctx:rng_pick(pool)
	if lucky > 0 then
		for _ = 1, lucky do
			chosen = better(ctx, chosen, ctx:rng_pick(pool))
		end
	end
	return { chosen.id }
end

-- A `cards` function cannot answer an empty list (an empty table crosses as a record, which
-- `for_each_card` refuses), so an empty board, which the hook sees as the list does, hands over its
-- empty list outright; the picks are otherwise drawn as the list reaches them.
local base = {
	cry = function(ctx: J.EffectContext)
		local cards: J.CardsSource = if #ctx:cards_in_scope({ side = "enemy" }) == 0 then {} else "each_cards"
		return {
			J.for_each_card({ cards = cards, each = "each_card" }),
		}
	end,
	each_cards = pick,
	each_card = function(id: string)
		return { J.destroy({ target = { of = "instance", instanceId = id } }) }
	end,
}

-- The same script: Lucky 1 is the Radiant face's catalog keyword, which `pick` reads.
return {
	base = base,
	radiant = "base",
}
