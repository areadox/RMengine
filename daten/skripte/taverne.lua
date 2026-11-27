-- =====================================================================
--  Beispiel-Skripte für die Taverne "Zum Goldenen Eber"
-- =====================================================================
--  Skripte sind in Lua geschrieben. Die ganze Beschreibung steht in der
--  ANLEITUNG (Kapitel "Skripte"). Hier das Wichtigste:
--
--    befehl{ woerter = {...}, raum = "id", funktion = function(s, rest, raum) ... end }
--    bei_betreten("raum", function(s, raum) ... end)
--    bei_sagen("raum", function(s, text, raum) ... end)
--    spaeter(sekunden, function() ... end)
--
--  s ist der Spieler: s.name, s.geld, s.merker ... und s:sende("..."),
--  s:andere("..."), s:gib("id"), s:geld_aendern(10), s:merke("name", wert) ...
-- =====================================================================

-- "kriech unter den tisch": Unter einem Tisch liegt eine Münze –
-- aber jeder Spieler findet sie nur einmal.
befehl{
  woerter = {"kriech", "krieche", "kriechen"},
  raum    = "taverne",
  syntax  = "kriech unter den tisch",
  hilfe   = "Unter einen der Tische kriechen",
  funktion = function(s, rest, raum)
    if not rest:find("tisch") then
      s:sende("Wohin willst du kriechen? Unter einen Tisch vielleicht?")
      return
    end
    s:sende("Du gehst auf die Knie und kriechst unter einen der schweren Eichentische.")
    s:andere(s.name .. " kriecht unter einen der Tische.")
    if s.merker.taverne_muenze then
      s:sende("Außer Krümeln und einem alten Korken ist hier nichts.")
    else
      s:sende("Zwischen den Dielen glänzt etwas – eine Silbermünze! Du steckst sie ein.")
      s:geld_aendern(10)
      s:merke("taverne_muenze", true)
    end
  end,
}

-- Der Wirt erkennt Stammgäste: Beim fünften Besuch gibt es ein Bier aufs Haus.
bei_betreten("taverne", function(s, raum)
  local besuche = (s.merker.taverne_besuche or 0) + 1
  s:merke("taverne_besuche", besuche)
  if besuche == 5 then
    spaeter(2, function()
      s:sende('Der Wirt nickt dir zu: „Ah, ein Stammgast! Das hier geht aufs Haus.“')
      s:gib("bier")
    end)
  end
end)

-- Wer in der Taverne "Prost" sagt, bekommt eine Antwort vom Wirt.
bei_sagen("taverne", function(s, text, raum)
  if text:lower():find("prost") then
    spaeter(1, function()
      welt.sende_raum("taverne", 'Der dicke Wirt hebt ein Glas: „Zum Wohl, ' .. s.name .. '!“')
    end)
  end
end)
