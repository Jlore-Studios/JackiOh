--!strict
local J = require("@jackioh")

-- C+ #71 Book of Buff, the first mode.
local BUFF_BOOK = "classicplus-071"
-- C+ #72 Book of Nerf, the second mode.
local NERF_BOOK = "classicplus-072"

-- The two modes, worded as the faces print them. The same labels on both faces.
local BUFF = "Book of Buff"
local NERF = "Book of Nerf"

-- The resume step the mode prompt re-enters (§10.6).
local STEP_BOOK = "book"

-- The Radiant face's Book is Radiant; the option labels are the same on both faces.
local function playtester(radiant: boolean)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			return {
				J.choose_mode({
					options = { BUFF, NERF },
					step = STEP_BOOK,
					prompt = "Playtester: choose one",
				}),
			}
		end,
		resume = {
			[STEP_BOOK] = function(ctx: J.EffectContext)
				local picked = ctx:chosen_options()[1]
				local id = BUFF_BOOK
				if picked == NERF then
					id = NERF_BOOK
				end
				local effects: { J.EffectCall } = {}
				for _ = 1, math.max(ctx:param("books"), 0) do
					effects[#effects + 1] = J.add_to_hand({ defId = id, radiant = radiant })
				end
				return effects
			end,
		},
	}
end

return {
	base = playtester(false),
	radiant = playtester(true),
}
