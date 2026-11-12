import QtQuick
import Quickshell.Io
import qs.Ui

// O placar do dia do vamo_fogao na barra do Omarchy.
//
// Roda `vamo barra --json` a cada meio minuto e mostra o campo `text`, com
// a dica (`tooltip`) e a cor que vem da classe. O clique esquerdo mostra e
// esconde a TUI, o direito relê o placar na hora.
//
// O formato do JSON está em book/src/a-barra.md. O módulo só confia no
// formato da versão que conhece (FORMAT_VERSION abaixo): se o binário
// mudar de versão, a barra avisa em vez de mostrar coisa errada.
//
// Configuração, na entrada do módulo no ~/.config/omarchy/shell.json (tudo
// opcional):
//   vamo      o comando do vamo (padrão: vamo, que precisa estar no PATH)
//   interval  de quantos em quantos segundos relê (padrão: 30)
//   onClick   o que o clique esquerdo roda (padrão: o workspace especial
//             vamo, o mesmo do extras/hyprland/vamo_fogao.lua)
//   colors    a cor de cada classe: vazio, andando, fechado e atraso
Item {
  id: root

  property var bar
  property string moduleName
  property var settings

  readonly property int formatVersion: 1
  property var output: ({})

  function setting(name, fallback) {
    var value = settings ? settings[name] : undefined
    return value === undefined || value === null ? fallback : value
  }

  // As cores do r4ul.com.br. Vermelho não entra: atraso é amarelo plasma.
  readonly property var defaultColors: ({
    vazio: "#666666",
    andando: "#e0e0e0",
    fechado: "#00ff00",
    atraso: "#ffff00"
  })

  // A classe que manda na cor: o atraso ganha do estado do placar.
  function classColor(klass) {
    var colors = Object.assign({}, defaultColors, setting("colors", {}))
    var classes = Array.isArray(klass) ? klass : [klass]
    if (classes.indexOf("atraso") !== -1) return colors.atraso
    for (var i = 0; i < classes.length; i++) {
      if (colors[classes[i]]) return colors[classes[i]]
    }
    return root.bar ? root.bar.barForeground : "white"
  }

  function read(text) {
    try {
      var parsed = JSON.parse(String(text || ""))
      if (parsed.versao !== root.formatVersion) {
        return {
          text: "vamo ?",
          tooltip: "O vamo mudou o formato da barra (versão " + parsed.versao
            + "). Atualize o vamo_fogao.qml.",
          class: ["atraso"]
        }
      }
      return parsed
    } catch (e) {
      // Sem JSON, o vamo deu erro (ou não está no PATH): a barra mostra o
      // que veio, pra dar pra ver o motivo na dica.
      return { text: "vamo !", tooltip: String(text || "o vamo não respondeu").trim(), class: ["atraso"] }
    }
  }

  function refresh() {
    if (!proc.running) proc.running = true
  }

  implicitWidth: button.implicitWidth
  implicitHeight: bar ? bar.barSize : 26

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.output.text || ""
    tooltipText: root.output.tooltip || ""
    foreground: root.classColor(root.output.class || [])
    onPressed: function(button) {
      if (button === Qt.RightButton) {
        root.refresh()
      } else if (button === Qt.LeftButton && root.bar) {
        root.bar.run(String(root.setting("onClick",
          "hyprctl dispatch 'hl.dsp.workspace.toggle_special(\"vamo\")'")))
        root.refresh()
      }
    }
  }

  Process {
    id: proc
    command: ["bash", "-lc", String(root.setting("vamo", "vamo")) + " barra --json 2>&1"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: root.output = root.read(text)
    }
  }

  Timer {
    interval: Number(root.setting("interval", 30)) * 1000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }
}
