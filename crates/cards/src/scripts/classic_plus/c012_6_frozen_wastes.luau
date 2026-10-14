--!strict
local J = require("@jackioh")

-- R280: the words the preview's value follows.
local PREVIEW_LABEL = "for each one destroyed"

-- The Units a destroy of all Units dooms now: every acting Unit but the Indestructible ones (R46).
-- `active_units_of` and `unit_view` are bound on every context (#464 gap 3); the condition
-- context's type does not list them, so the preview passes its context as `any`.
local function doomed(ctx: any): number
	local count = 0
	for _, player in ipairs({ "p1", "p2" }) do
		for _, unit in ipairs(ctx:active_units_of(player)) do
			local indestructible = false
			for _, keyword in ipairs(ctx:unit_view(unit.id).keywords) do
				if keyword.kind == "Indestructible" then
					indestructible = true
				end
			end
			if not indestructible then
				count += 1
			end
		end
	end
	return count
end

-- The face's deck: your own on the base face, your opponent's on the Radiant face.
local function frozen_wastes(enemy_deck: boolean)
	local function deck_of(ctx: any): J.PlayerId
		local me = ctx:controller()
		return if enemy_deck then ctx:opponent_of(me) else me
	end
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.destroy_all({ side = "any" }),
				-- "Each one" is the Units the destroy dooms, read as it resolves; the exile follows in
				-- the same list, before the state check (R59), and a short deck exiles what it has.
				J.for_each_card({
					cards = "each_cards",
					each = "each_card",
					data = { count = doomed(ctx), deck = deck_of(ctx) },
				}),
			}
		end,
		each_cards = function(ctx: J.EffectContext, data: any)
			local ids: { string } = {}
			for at, card in ipairs(ctx:zone_cards(data.deck, "library")) do
				if at > math.max(data.count, 0) then
					break
				end
				ids[#ids + 1] = card.id
			end
			return ids
		end,
		each_card = function(id: string)
			return { J.exile({ target = { of = "instance", instanceId = id } }) }
		end,
		preview = function(ctx: J.ConditionContext)
			local value = math.min(doomed(ctx), ctx:zone_count(deck_of(ctx), "library"))
			return {
				{ label = PREVIEW_LABEL, value = value },
			}
		end,
	}
end

return {
	base = frozen_wastes(false),
	radiant = frozen_wastes(true),
}
