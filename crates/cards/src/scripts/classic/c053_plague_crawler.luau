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

local base = {
	-- "another permanent": the top of any unit pile or any backrow card, either side, never this one.
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit", "backrow" },
				excludeSelf = true,
			},
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague({ target = { of = "chosen" }, amount = ctx:param("tokens") }),
		}
	end,
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
