--!strict
local J = require("@jackioh")

-- Its four keywords are printed (§10.4's keyword layer, §6.1). Patch v0.3.9 (#831): "Paid (3): Equip."
-- Equip holds only on a card stamped embiggened, so it is offered only at the (3) price (R831013).
local function paid_equip(ctx: J.HookArgs): { J.Keyword }
	local me = ctx:self_()
	if type(me) == "table" and me.embiggened == true then
		return { { kind = "Equip" } }
	end
	return {}
end

-- One script: the Radiant face adds Reborn to the printed list, and pays for Equip the same way.
return {
	base = { conditionalKeywords = paid_equip },
	radiant = "base",
}
