--!strict
local J = require("@jackioh")

-- The step name the delayed effect carries; it labels the pause.
local EXILE_STEP = "exileHand"

-- R62: the end-of-turn delayed effect. It runs after the trap window and before cleanup, and it
-- exiles whatever the hand holds then.
local function exile_the_hand(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.exile_hand({ player = "self" }),
	}
end

-- The Radiant face's rider: each card played this turn draws 1 (§10.5 step 5).
local function combo_draw(ctx: J.EffectContext): J.EffectCall
	return J.add_player_modifier({
		player = "self",
		mod = {
			kind = "comboDraw",
			amount = ctx:param("comboDraw"),
			expiry = { ["until"] = "thisTurn", turn = ctx:turn().number },
		},
	})
end

-- The two faces differ only in whether the turn's cards gain the Combo draw.
local function fullsend(with_combo_draw: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {
				J.refresh_mana({ amount = ctx:param("refresh") }),
				J.add_player_modifier({
					player = "self",
					-- "your cards", so no `onlyType`; R65 applies it as a flat discount.
					mod = {
						kind = "costDiscount",
						amount = ctx:param("discount"),
						expiry = { ["until"] = "thisTurn", turn = ctx:turn().number },
					},
				}),
			}
			if with_combo_draw then
				effects[#effects + 1] = combo_draw(ctx)
			end
			-- R62: "my own end of turn" is "self".
			effects[#effects + 1] = J.delay({
				at = { phase = "end", player = "self" },
				step = EXILE_STEP,
				hook = "resume",
			})
			return effects
		end,
		-- The one registration (R126): the step table the `delay` above names.
		resume = {
			[EXILE_STEP] = exile_the_hand,
		},
	}
end

return {
	base = fullsend(false),
	radiant = fullsend(true),
}
