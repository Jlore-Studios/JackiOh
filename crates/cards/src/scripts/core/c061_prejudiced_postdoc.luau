--!strict
local J = require("@jackioh")

-- The one difference between the faces is the pick's filter: base narrows it to Humans, radiant
-- does not.
local function postdoc(filter: J.TargetFilter)
	return {
		targets = {
			{ kind = "target", min = 1, max = 1, filter = filter },
		},
		cry = function(ctx: J.EffectContext)
			return {
				J.summon_copy({
					of = { of = "chosen" },
					-- §8.3: the copy has no text and no granted keywords of its own.
					vanilla = true,
					grantedKeywords = false,
				}),
			}
		end,
	}
end

return {
	-- "on the field" and §8's Conventions: either side unless the cell narrows it.
	base = postdoc({ side = "any", of = { "unit" }, tags = { "Human" } }),
	radiant = postdoc({ side = "any", of = { "unit" } }),
}
