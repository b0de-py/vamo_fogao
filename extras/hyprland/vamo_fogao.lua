-- O vamo_fogao à mão no Omarchy: SUPER + ALT + V mostra e esconde a TUI.
--
-- É a versão em Lua do vamo_fogao.conf, pro Hyprland com a config em Lua
-- do Omarchy (o ~/.config/hypr/hyprland.lua). Os `o.bind`, `o.launch` e
-- `o.window` são ajudantes do Omarchy; fora dele, use `hl.bind` e
-- `hl.window_rule` direto.
--
-- A TUI mora num workspace especial, o `special:vamo`. Esconder o
-- workspace não fecha o processo, então a tela volta do jeito que estava.
-- Na primeira vez que o workspace abre vazio, abre a TUI no Alacritty.
--
-- Pra usar:
--
--   1. Instale o `vamo` no PATH: cargo install --path crates/vamo_fogao
--   2. Copie este arquivo pra ~/.config/hypr/vamo_fogao.lua
--   3. Ponha no fim do ~/.config/hypr/hyprland.lua:
--        require("hypr.vamo_fogao")
--   4. Recarregue com hyprctl reload e confira com hyprctl configerrors
--
-- Testado no Hyprland 0.56.2 com o Omarchy.

local VAMO = "special:vamo"

-- A letra da janela do vamo. O terminal costuma ter letra pequena, boa pra
-- código e ruim pra uma tela que se lê de relance. O `-o` do Alacritty troca
-- a config só nesta janela; o resto do terminal fica como está.
--
-- Pra usar também a fonte do r4ul.com.br, instale a Space Mono (no Arch, o
-- pacote ttf-space-mono-nerd) e troque a linha por:
--
--   local LETRA = "-o font.size=11 -o 'font.normal.family=\"SpaceMono Nerd Font\"'"
--
-- No Kitty, o equivalente é `-o font_size=11`; no foot, `-o font=monospace:size=11`.
local LETRA = "-o font.size=11"

-- O atalho. No Omarchy, o SUPER + ALT + F já é o "Full width". Antes de
-- trocar a tecla, confira se a nova está livre: hyprctl binds
o.bind("SUPER + ALT + V", "vamo_fogao", hl.dsp.workspace.toggle_special("vamo"))

-- A primeira abertura: com o workspace vazio, abre o terminal com a TUI.
-- A classe `vamo_fogao` é o que a regra de janela abaixo procura. No
-- Alacritty e no Kitty é `--class vamo_fogao`; no foot, `--app-id
-- vamo_fogao`.
hl.on("workspace.created", function(ws)
  if ws and ws.name == VAMO and ws.is_empty then
    hl.exec_cmd(o.launch("alacritty --class vamo_fogao " .. LETRA .. " -e vamo tui"), { workspace = VAMO .. " silent" })
  end
end)

-- O tamanho: uma janela flutuante grande, no meio da tela, por cima do que
-- estiver aberto. Pra tela cheia, comente esta linha: sozinha no workspace
-- especial, a janela ocupa a tela toda.
o.window("^(vamo_fogao)$", { float = true, center = true, size = { "monitor_w*0.9", "monitor_h*0.85" } })
