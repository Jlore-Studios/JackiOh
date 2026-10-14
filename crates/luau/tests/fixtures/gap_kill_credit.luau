--!strict
local J = require("@jackioh")

-- Gap 9: `with_kill_credit`'s `pairs` and `then` by name, with data. The
-- pairs name every enemy unit beside the killer's victim id from `data`;
-- `then` answers one ping per pair from the pair the engine hands it.
local base = {
	cry = function(_ctx: J.EffectContext)
		return {
			J.with_kill_credit({
				killer = { of = "selfHero" },
				pairs = "kill_pairs",
				-- `then` is a Luau keyword, so the key is bracketed; `pairs` stays bare.
				["then"] = "kill_then",
				data = { ping = 1 },
				during = J.gain_mana({ amount = 0 }),
			}),
		}
	end,
	kill_pairs = function(ctx: J.EffectContext, _data: any)
		local credits = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			credits[#credits + 1] = { victimId = unit.id, toId = unit.id }
		end
		return credits
	end,
	kill_then = function(pair: any, data: any)
		return { J.damage({ to = { of = "instance", instanceId = pair.toId }, amount = data.ping }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
