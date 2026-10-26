--!strict
local J = require("@jackioh")

-- #1904: `choose_target_where`'s `where` by name. The named `admits` is asked
-- for each card the scope offers (nil for a hero), with a hook-args `ctx`, and
-- admits only an enemy `lx-fx-002`: of the two enemy units the scope offers,
-- the prompt holds one. The Radiant face's verb misspells the name, so its cry
-- fails as it builds.
local base = {
	cry = function(_ctx: J.EffectContext)
		return {
			J.choose_target_where({
				step = "picked",
				scope = { side = "enemy", of = { "unit" } },
				where = "admits",
			}),
		}
	end,
	admits = function(ctx: J.HookArgs, card: any): boolean
		return card ~= nil and card.controller ~= ctx:controller() and card.defId == "lx-fx-002"
	end,
}

local radiant = {
	cry = function(_ctx: J.EffectContext)
		return {
			J.choose_target_where({
				step = "picked",
				scope = { side = "enemy", of = { "unit" } },
				where = "admit",
			}),
		}
	end,
	admits = base.admits,
}

return {
	base = base,
	radiant = radiant,
}
