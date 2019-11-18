/c local map={}
local comb = game.player.selected
count = 0
index = 0
function addSig(sig)
  if count % 18 == 0 then
    comb = game.player.surface.create_entity({name = "constant-combinator", position = {x=comb.position.x+1,y=comb.position.y}, force = game.forces.player})
  end
  comb.get_control_behavior().set_signal(index % 18 + 1, {signal=sig, count=count + 1})
  count = count + 1
  index = index + 1
  return
end
for _,v in pairs(game.virtual_signal_prototypes) do
  if (not v.special) and v.name~="signal-black" then
    addSig({name=v.name,type="virtual"})
  end
end
for _,f in pairs(game.fluid_prototypes) do
  if not f.hidden then
    addSig({name=f.name,type="fluid"})
  end
end
for _,i in pairs(game.item_prototypes) do
  if not i.has_flag("hidden") and i.name~="magic-lamp" then
   addSig({name=i.name,type="item"})
  end
end
