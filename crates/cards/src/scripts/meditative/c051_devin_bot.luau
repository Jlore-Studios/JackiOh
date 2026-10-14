--!strict
local J = require("@jackioh")

-- R1080: every empty, unlocked unit zone, left to right, gets its own pick from the Units whose
-- printed base face is 1/1, tokens included, never Devin Bot itself (R387).
local function devin_bot(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.fill_board_random({
					query = {
						type = "Unit",
						stats = { attack = 1, health = 1 },
						withTokens = true,
					},
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = devin_bot(false),
	radiant = devin_bot(true),
}
