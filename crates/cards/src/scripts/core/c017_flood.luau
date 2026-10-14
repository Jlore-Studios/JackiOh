--!strict
local J = require("@jackioh")

-- The three options, word for word from §8. They are the card's public interface: `legal_actions`
-- offers these strings, the client shows them and the hook below matches on them, so the
-- declaration and the hook must never drift apart — hence one constant for both.
local BOUNCE_ALL = "bounce all units"
local BOUNCE_ENEMY = "bounce all enemy units"
local DESTROY_ENEMY = "destroy all enemy units"

-- "Bounce all units on both sides" — the base text, and radiant's first option.
local function bounce_both_sides(): J.EffectCall
	return J.bounce_all({ side = "any" })
end

-- The option the play carried. `why_choices_refused` (play_choices.rs) already refuses a play that
-- names no mode for a declared one, so the fallback is unreachable in a legal game; it is the first
-- option rather than "do nothing" because "choose one" is a mandatory choice, not an optional rider.
local function picked_mode(ctx: J.EffectContext): string
	local picked = ctx:chosen_options()[1]
	if picked == nil then
		return BOUNCE_ALL
	end
	return picked
end

local function chosen_effect(ctx: J.EffectContext): J.EffectCall
	local picked = picked_mode(ctx)
	if picked == DESTROY_ENEMY then
		return J.destroy_all({ side = "enemy", rows = { "units" } })
	end
	if picked == BOUNCE_ENEMY then
		return J.bounce_all({ side = "enemy" })
	end
	return bounce_both_sides()
end

local base = {
	cry = function(ctx: J.EffectContext)
		return { bounce_both_sides() }
	end,
}

local radiant = {
	modes = {
		{ kind = "mode", options = { BOUNCE_ALL, BOUNCE_ENEMY, DESTROY_ENEMY } },
	},
	-- "; then draw 1": the draw follows whichever mode resolved, on all three of them. Its count is
	-- the declared number `draw` (R386).
	cry = function(ctx: J.EffectContext)
		local count = ctx:param("draw")
		return { chosen_effect(ctx), J.draw({ count = count }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
