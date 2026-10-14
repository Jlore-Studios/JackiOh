--!strict
local J = require("@jackioh")

-- §8: "Deal 2 damage", radiant "Deal 4 damage": the declared number `damage` (R386).
local function combo_fodder()
	return {
		-- R81: "a target" — any unit or hero on either side, picked with the play.
		targets = {
			{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "hero" } } },
		},
		cry = function(ctx: J.EffectContext)
			local amount = ctx:param("damage")
			return {
				J.damage({ to = { of = "chosen" }, amount = amount, lifesteal = true }),
			}
		end,
	}
end

return {
	base = combo_fodder(),
	radiant = combo_fodder(),
}
