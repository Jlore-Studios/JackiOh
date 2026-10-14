--!strict
local J = require("@jackioh")

-- §7: the CN-Virus it shuffles in.
local CN_VIRUS = "core-090-1"

-- "Whenever this destroys a Unit, shuffle {viruses} CN-Virus into your opponent's deck": on each
-- `destroyed` whose `killerId` is this (R42), the opponent's cards, Radiant on the Radiant face; a
-- full deck refuses them (R80).
local function on_kill(radiant: boolean)
	return {
		id = "cn-golem-kill",
		on = { "destroyed" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "destroyed" or event.killerId == nil then
				return {}
			end
			local me = ctx:self_()
			if me == nil or event.killerId ~= me.id then
				return {}
			end
			return {
				J.shuffle_into({ defId = CN_VIRUS, count = ctx:param("viruses"), player = "enemy", radiant = radiant }),
			}
		end,
	}
end

return {
	base = {
		triggers = { on_kill(false) },
	},
	radiant = {
		triggers = { on_kill(true) },
	},
}
