--!strict
local J = require("@jackioh")

-- A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's
-- controller's hero.
local function attacks_your_hero(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "attackDeclared" then
		return false
	end
	if event.forced then
		return false
	end
	local you = ctx:controller()
	local attacker = ctx:find_instance(event.attackerId)
	if type(attacker) ~= "table" or attacker.controller ~= ctx:opponent_of(you) then
		return false
	end
	local target = ctx:attack_target_of(event.targetId)
	return type(target) == "table" and target.kind == "hero" and target.player == you
end

local function groom_shroom(radiant: boolean)
	return {
		triggers = {
			{
				id = "groom-shroom",
				on = { "attackDeclared" },
				when = attacks_your_hero,
				run = function(ctx: J.EffectContext, event: J.GameEvent)
					local lanes: { number } = {}
					for _, zone in ipairs(ctx:fill_board_zones(ctx:controller())) do
						lanes[#lanes + 1] = zone.lane
					end
					local effects: { J.EffectCall } = {}
					for _, lane in ipairs(lanes) do
						effects[#effects + 1] = J.summon_random({
							query = { type = "Unit", tags = { "Felinor" } },
							lane = lane,
							radiant = radiant,
						})
					end
					-- "Give them Taunt": the Units now standing in the zones the fill took. The
					-- lanes travel in `data`: the `cards` function runs when the list reaches it,
					-- after the summons, in a fresh chunk.
					effects[#effects + 1] = J.for_each_card({
						cards = "each_cards",
						each = "each_card",
						data = { lanes = lanes },
					})
					return effects
				end,
			},
		},
		each_cards = function(ctx: J.EffectContext, data: any)
			if type(data) ~= "table" or type(data.lanes) ~= "table" then
				return {}
			end
			local me = ctx:controller()
			local ids: { string } = {}
			for _, lane in ipairs(data.lanes) do
				if type(lane) == "number" then
					local card = ctx:card_at(me, lane)
					if type(card) == "table" then
						ids[#ids + 1] = card.id
					end
				end
			end
			return ids
		end,
		each_card = function(id: string)
			return {
				J.grant_keyword({
					target = { of = "instance", instanceId = id },
					keyword = { kind = "Taunt" },
				}),
			}
		end,
	}
end

return {
	base = groom_shroom(false),
	radiant = groom_shroom(true),
}
