--!strict
local J = require("@jackioh")

-- "Death: Steal the Unit that destroyed this" — identical on both faces. R42's killer, read off Mrow's
-- own snapshot (R78, R361; `killer_of_self`, #464 gap 9): only a Unit acting on the field answers.
local base = {
	death = function(ctx: J.EffectContext)
		local killer = ctx:killer_of_self()
		if killer == nil then
			return {}
		end
		return { J.steal({ instanceId = killer.id }) }
	end,
}

-- The faces differ only in the printed face, which is the catalog's.
return {
	base = base,
	radiant = "base",
}
