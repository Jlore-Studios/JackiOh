--!strict
local J = require("@jackioh")

-- Declarations as data: targets, modes, activations, triggers in every zone,
-- one replacement, target checks, resume and grants.
local base = {
	targets = {
		{ kind = "target", min = 1, max = 1 },
	},
	modes = {
		{ kind = "mode", options = { "a", "b" } },
	},
	staticFlags = {
		quickdraw = true,
	},
	activations = {
		{
			id = "ping",
			label = "Ping",
			uses = 1,
			canActivate = function(ctx: J.ConditionContext)
				return true
			end,
			run = function(ctx: J.EffectContext)
				return {
					J.damage({ to = { of = "enemyHero" }, amount = 1 }),
				}
			end,
		},
	},
	triggers = {
		{
			id = "t1",
			on = { "turnStarted" },
			when = function(ctx: J.EffectContext, event: J.GameEvent)
				return true
			end,
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				return {
					J.draw({ count = 1 }),
				}
			end,
		},
	},
	handTriggers = {
		{
			id = "h1",
			on = { "turnEnded" },
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				return {
					J.gain_mana({ amount = 1 }),
				}
			end,
		},
	},
	replacements = {
		{
			id = "r1",
			on = "healed",
			instead = { damage = "pierce" },
		},
	},
	targetChecks = {
		cheap = function(args)
			return true
		end,
	},
	resume = {
		step_two = function(ctx: J.EffectContext)
			return {
				J.draw({ count = 1 }),
			}
		end,
	},
	grants = {
		extra = function(ctx: J.EffectContext)
			return {
				J.gain_mana({ amount = 1 }),
			}
		end,
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
