--!strict
local J = require("@jackioh")

local CHOSEN: J.TargetSpec = { of = "chosen" }

-- What the chosen permanent's side and row make of the removed Plague Counter, read as the
-- activation runs: an enemy permanent is exiled (Radiant: fused onto a card of yours of its type, or
-- exiled if you have none); your backrow card draws; your Unit attacks a random enemy.
local function outcome(ctx: J.EffectContext, fuses: boolean): { J.EffectCall }
	local card = ctx:instance_of(CHOSEN)
	if card == nil or card.zone.z ~= "field" then
		return {}
	end
	if card.controller ~= ctx:controller() then
		if fuses then
			return { J.fuse_onto_your_card({ target = CHOSEN }) }
		end
		return { J.exile({ target = CHOSEN }) }
	end
	if card.zone.row == "backrow" then
		return { J.draw({ count = ctx:param("draw") }) }
	end
	return { J.forced_attack_random({ attacker = CHOSEN, times = ctx:param("attacks") }) }
end

local function mutate(fuses: boolean)
	return {
		activations = {
			{
				id = "mutate",
				label = "Remove a Plague Counter",
				uses = "unlimited",
				targets = {
					{
						kind = "target",
						min = 1,
						max = 1,
						filter = { side = "any", of = { "unit", "backrow" }, plague = true },
					},
				},
				canActivate = function(ctx: J.ConditionContext)
					for _, card in ipairs(ctx:permanents_on_field(ctx:controller())) do
						if ctx:plague_on(card.id) > 0 then
							return true
						end
					end
					return false
				end,
				run = function(ctx: J.EffectContext)
					local effects = { J.consume_plague({ target = CHOSEN }) }
					for _, effect in ipairs(outcome(ctx, fuses)) do
						effects[#effects + 1] = effect
					end
					return effects
				end,
			},
		},
	}
end

return {
	base = mutate(false),
	radiant = mutate(true),
}
