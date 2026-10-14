--!strict
local J = require("@jackioh")

-- The radiant face is the same card at doubled numbers, so one script carries both: the declared
-- numbers `buff` (3, 6) and `keywords` (1, 2), R386.
local base = {
	-- "Target unit": either side, §8's Conventions, and no narrowing in either cell.
	targets = {
		-- R656: a buff and a keyword help, so a random cast that targets enemies aims this at friends.
		-- R703: and the play needs it, so no Unit to target means no play.
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
			aim = "help",
			required = true,
		},
	},
	cry = function(ctx: J.EffectContext)
		local stat = ctx:param("buff")
		local keywords = ctx:param("keywords")
		return {
			J.buff({ target = { of = "chosen" }, attack = stat, health = stat }),
			J.grant_random_keywords({ target = { of = "chosen" }, count = keywords }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
