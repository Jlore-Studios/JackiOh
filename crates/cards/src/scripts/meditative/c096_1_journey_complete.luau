--!strict
local J = require("@jackioh")

-- What M #96 Meditative Journey writes (R1246).
local JOURNEY_KEY = "journey"

-- The ids M #96 wrote on this card, in order; none for one that remembers nothing (R1248).
local function went(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	local me = ctx:live_self()
	if me == nil or me.memory == nil then
		return ids
	end
	local journey = me.memory[JOURNEY_KEY]
	if type(journey) ~= "table" then
		return ids
	end
	for _, id in ipairs(journey) do
		if type(id) == "string" then
			ids[#ids + 1] = id
		end
	end
	return ids
end

-- Cast on draw (R40, R70): each card that went on the journey and is still in an exile pile returns
-- to its owner's hand (R746), Radiant (§6.3 Make Radiant, R74); on the Radiant face it costs
-- `discount` less, a price that lands after R766 reset it in exile. The hand cap burns the rest.
local function journey_complete(radiant: boolean)
	return {
		staticFlags = { castOnDraw = true },
		cry = function(ctx: J.EffectContext)
			local discount = ctx:param("discount")
			local effects = {}
			for _, id in ipairs(went(ctx)) do
				local card = ctx:find_instance(id)
				if card ~= nil and card.zone.z == "exile" then
					local instance: J.TargetSpec = { of = "instance", instanceId = id }
					local args: J.AddToHandArgs = { instance = instance, radiant = true }
					if radiant then
						args.costMod = -discount
					end
					effects[#effects + 1] = J.add_to_hand(args)
				end
			end
			return effects
		end,
	}
end

return {
	base = journey_complete(false),
	radiant = journey_complete(true),
}
