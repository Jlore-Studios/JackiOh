--!strict
local J = require("@jackioh")

-- A placement of Plague Counters on this card: `counterChanged` for `plague` with `placed`, naming it.
local function placed_on_this(ctx: J.EffectContext, event: J.GameEvent): boolean
	local me = ctx:self_()
	if type(me) ~= "table" then
		return false
	end
	if event.type ~= "counterChanged" then
		return false
	end
	return event.counter == "plague" and event.instanceId == me.id and (event.placed or 0) > 0
end

-- The Cry's resume step: the permanent picked as it resolves.
local PLACED_STEP = "placed"

local base = {
	-- "a permanent": the top of any unit pile or any backrow card, either side, this one included
	-- (patch v0.3.9, #831; "another permanent", a declared target, before), so the pick is a prompt
	-- made once this stands on the field (R831016).
	cry = function(ctx: J.EffectContext)
		return {
			J.choose_target({
				step = PLACED_STEP,
				scope = { side = "any", of = { "unit", "backrow" } },
				prompt = "Plague Crawler: place Plague Counters on a permanent",
			}),
		}
	end,
	resume = {
		[PLACED_STEP] = function(ctx: J.EffectContext)
			return {
				J.place_plague({ target = { of = "chosen" }, amount = ctx:param("tokens") }),
			}
		end,
	},
	triggers = {
		{
			id = "plague-crawler-placed",
			on = { "counterChanged" },
			when = placed_on_this,
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if not placed_on_this(ctx, event) then
					return {}
				end
				return {
					J.draw({ count = ctx:param("draw") }),
				}
			end,
		},
	},
}

-- The same script: the Radiant face's 2 tokens and draw 2 are its declared numbers, which `param`
-- reads off the running face.
return {
	base = base,
	radiant = "base",
}
