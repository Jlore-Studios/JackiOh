--!strict
local J = require("@jackioh")

-- M #96.1 Journey Complete, which reads the same key (R1246).
local JOURNEY_COMPLETE = "meditative-096-1"
local JOURNEY_KEY = "journey"
local STEP = "journey"

-- The pick prompt's answer, in the order picked (R221). There is no `targets` reader, so the picks
-- are probed as Classic #11 Mind Melt probes them: `instance_of` answers each picked hand card and
-- nil past the end. A hand card is on no stay a departure could end, so every pick answers.
local function picked_ids(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	local index = 0
	while true do
		local spec: J.TargetSpec = { of = "chosen", index = index }
		local card = ctx:instance_of(spec)
		if type(card) ~= "table" then
			return ids
		end
		ids[#ids + 1] = card.id
		index += 1
	end
end

local function journey(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {
				J.choose_pick({
					step = STEP,
					from = { { zone = "hand" } },
					max = ctx:param("cards"),
					prompt = "Choose cards in your hand to go on a journey",
				}),
			}
			-- The Radiant face's draw follows the step (R113), or comes at once with an empty hand.
			if radiant then
				effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
			end
			return effects
		end,
		resume = {
			[STEP] = function(ctx: J.EffectContext)
				local picked = picked_ids(ctx)
				if #picked == 0 then
					return {}
				end
				local effects: { J.EffectCall } = {}
				for _, id in ipairs(picked) do
					effects[#effects + 1] = J.exile({ target = { of = "instance", instanceId = id } })
				end
				effects[#effects + 1] = J.shuffle_into({
					defId = JOURNEY_COMPLETE,
					count = 1,
					radiant = radiant,
					memory = { [JOURNEY_KEY] = picked },
				})
				return effects
			end,
		},
	}
end

return {
	base = journey(false),
	radiant = journey(true),
}
