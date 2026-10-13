--!strict
local J = require("@jackioh")

-- A spell's script is its `cry` hook (§10.9). §7: base gains 1, radiant 2 — the declared
-- number `mana` (R386), read off the face that is running.
--
-- The two faces spell their `cry` out separately, not through one factory: Luau reuses one
-- closure object for every evaluation of an upvalue-free literal (#464 gap 14), so one
-- shared local — or one factory called twice — would give both faces one adapter, and the
-- faces keep their own (`!ptr_eq`).
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.gain_mana({ amount = ctx:param("mana") }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.gain_mana({ amount = ctx:param("mana") }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
