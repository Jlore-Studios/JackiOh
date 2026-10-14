--!strict
local J = require("@jackioh")

-- R280: the formula as both faces print it.
local DOCTOR_LABEL = "the number of Plague Counters on the field"

-- Every Plague Counter on the field: each permanent on both sides, face-down ones included, the top
-- of each pile only (R13). Walked from the controller's side, so the read never asks whose turn it
-- is. `permanents_on_field` and `plague_multiplier_of` are bound on every context (#464 gap 3); the
-- condition context's type does not list them, so the preview passes its context as `any`.
local function tokens_on_field(ctx: any): number
	local sum = 0
	for _, card in ipairs(ctx:permanents_on_field(ctx:controller())) do
		sum += math.max(card.counters.plague or 0, 0)
	end
	return sum
end

-- The Plague Counters the Cry's own placement puts on the Doctor: the declared number times its own
-- multiplier (a Doctor fused onto a C #27 Pestilent Slime doubles it), in hand as it would once
-- played. Both faces place since patch v0.3.9 (#831), the Radiant face alone before.
local function tokens_it_places(ctx: any, me: any): number
	if me == nil then
		return 0
	end
	return math.max(ctx:param("tokens"), 0) * ctx:plague_multiplier_of(me.id)
end

-- N: every Plague Counter on the field, plus the Cry's own placement, as the Cry resolves now.
local function damage_now(ctx: any, me: any): number
	return tokens_on_field(ctx) + tokens_it_places(ctx, me)
end

-- The placement is the declared `tokens`: 1 on the base face, 2 on the Radiant face.
local base = {
	targets = {
		{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "hero" } } },
	},
	cry = function(ctx: J.EffectContext)
		-- The Cry only reads the board while it builds its list.
		local amount = damage_now(ctx, ctx:self_())
		local hit = {}
		if amount > 0 then
			hit[1] = J.damage({ to = { of = "chosen" }, amount = amount })
		end
		local effects = { J.place_plague({ target = { of = "self" }, amount = ctx:param("tokens") }) }
		for _, effect in ipairs(hit) do
			effects[#effects + 1] = effect
		end
		return effects
	end,
	preview = function(ctx: J.ConditionContext)
		return {
			{ label = DOCTOR_LABEL, value = damage_now(ctx, ctx:self_()) },
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
