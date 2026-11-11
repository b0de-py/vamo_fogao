-- O vamo_fogao à mão no Hyprland com config em Lua, a do Omarchy: um atalho
-- mostra e esconde a TUI. É o mesmo que o vamo_fogao.conf faz, no formato
-- que o Hyprland lê quando a config é o hyprland.lua.
--
-- A TUI mora num workspace especial, o `special:vamo`. Esconder o workspace
-- não fecha o processo, então a tela volta do jeito que estava. Na primeira
-- vez que o workspace abre vazio, o Hyprland abre a TUI.
--
-- Pra usar, copie este arquivo pra ~/.config/hypr/ e ponha no fim do
-- hyprland.lua:
--
--   require("hypr.vamo_fogao")
--
-- Depois confira com `hyprctl reload` e `hyprctl configerrors`.
--
-- `o.bind`, `o.launch` e `o.window` são ajudantes do Omarchy. Sem o Omarchy,
-- use `hl.bind` e `hl.window_rule` direto, ou o vamo_fogao.conf.
--
-- O `vamo` precisa estar no PATH: cargo install --path crates/vamo_fogao

local VAMO = "special:vamo"

-- O atalho. Antes de usar, confira se ele está livre: hyprctl binds
-- (no Omarchy, o SUPER ALT F já é o "Full width").
o.bind("SUPER + ALT + V", "vamo_fogao", hl.dsp.workspace.toggle_special("vamo"))

-- O terminal que abre a TUI. A classe `vamo_fogao` é o que a regra de janela
-- abaixo procura. No Alacritty e no Kitty é `--class vamo_fogao`; no foot,
-- `--app-id vamo_fogao`.
hl.on("workspace.created", function(ws)
  if ws and ws.name == VAMO and ws.is_empty then
    hl.exec_cmd(o.launch("alacritty --class vamo_fogao -e vamo tui"), { workspace = VAMO .. " silent" })
  end
end)

-- O tamanho: uma janela flutuante grande, no meio da tela. Pra tela cheia,
-- comente esta regra: sozinha no workspace especial, a janela ocupa a tela toda.
o.window("^(vamo_fogao)$", { float = true, center = true, size = { "monitor_w*0.9", "monitor_h*0.85" } })
