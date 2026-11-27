-- Die ganze Welt zählt mit: Jeder zehnte besiegte Wolf wird ausgerufen.
bei_sieg("wolf", function(s, monster, raum)
  local n = (welt.merker.woelfe or 0) + 1
  welt.merke("woelfe", n)
  if n % 10 == 0 then
    welt.sende_alle("Im Dorf erzählt man sich: " .. s.name .. " hat den " .. n .. ". Wolf erlegt!")
  end
end)
