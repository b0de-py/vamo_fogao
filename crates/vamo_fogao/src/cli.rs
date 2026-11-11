//! A linha de comando, descrita com o clap.
//!
//! O clap fala inglês, então os textos que ele gera sozinho (títulos da
//! ajuda, `--help`, `--version`, mensagens de erro de uso) são trocados por
//! textos em português aqui.

use clap::error::{ContextKind, ErrorKind};
use clap::{ArgAction, Parser, Subcommand};
use vamo_core::{Area, Fit, Status, TaskId};

use crate::messages::Voice;

/// O molde da ajuda, com os títulos em português.
const HELP_TEMPLATE: &str = "\
{about}

Uso: {usage}

{all-args}{after-help}
";

/// O molde da ajuda de cada comando, como `vamo add --help`.
const SUBCOMMAND_HELP_TEMPLATE: &str = "\
{about}

Uso: {usage}

{all-args}
";

/// O que aparece no fim de `vamo --help`.
const AFTER_HELP: &str = "\
Variáveis de ambiente:
  VAMO_LINGUAGEM  jogo ou neutra, o mesmo que --linguagem
  VAMO_MAQUINA    o nome desta máquina nos eventos (sem ela, vale o hostname)
  VAMO_LOG        o nível dos logs na saída de erro, como debug (padrão: warn)
  XDG_DATA_HOME   onde fica a pasta do banco (padrão: ~/.local/share)";

/// vamo, o app de tarefas do vamo_fogao
#[derive(Debug, Parser)]
#[command(
    name = "vamo",
    version,
    help_template = HELP_TEMPLATE,
    after_help = AFTER_HELP,
    override_usage = "vamo [OPÇÕES] <COMANDO>",
    subcommand_help_heading = "Comandos",
    subcommand_value_name = "COMANDO",
    next_help_heading = "Opções",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    /// O que fazer. Sem comando, mostra esta ajuda.
    #[command(subcommand)]
    pub(crate) command: Option<Command>,

    /// A linguagem das mensagens: jogo (o padrão) ou neutra
    #[arg(
        long = "linguagem",
        global = true,
        env = "VAMO_LINGUAGEM",
        value_name = "LINGUAGEM",
        default_value = "jogo",
        hide_default_value = true,
        hide_possible_values = true,
        hide_env = true
    )]
    pub(crate) voice: Voice,

    /// Mostra esta ajuda
    #[arg(short = 'h', long = "help", global = true, action = ArgAction::Help)]
    help: Option<bool>,

    /// Mostra a versão
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: Option<bool>,
}

/// Os comandos do `vamo`.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Põe uma tarefa nova no banco
    #[command(
        override_usage = "vamo add [OPÇÕES] <TÍTULO>...",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        next_help_heading = "Opções"
    )]
    Add {
        /// O título, no padrão do caderno: verbo na frente e até cinco
        /// palavras. Não precisa de aspas.
        #[arg(
            value_name = "TÍTULO",
            required = true,
            num_args = 1..,
            help_heading = "Argumentos"
        )]
        title: Vec<String>,

        /// A área da tarefa (sem ela, vai pra geral)
        #[arg(
            short = 'a',
            long = "area",
            value_name = "ÁREA",
            default_value = "geral",
            hide_default_value = true
        )]
        area: Area,

        /// O encaixe: foco, brecha ou com-alguém (sem ele, brecha)
        #[arg(
            short = 'e',
            long = "encaixe",
            value_name = "ENCAIXE",
            default_value = "brecha",
            hide_default_value = true
        )]
        fit: Fit,

        /// O status de partida (sem ele, pronta)
        #[arg(
            short = 's',
            long = "status",
            value_name = "STATUS",
            default_value = "pronta",
            hide_default_value = true
        )]
        status: Status,

        /// As notas: o detalhe que não cabe no título
        #[arg(short = 'n', long = "notas", value_name = "NOTAS")]
        notes: Option<String>,
    },

    /// Lista as tarefas em jogo (sem as concluídas e as descartadas)
    #[command(
        override_usage = "vamo list [OPÇÕES]",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        next_help_heading = "Opções"
    )]
    List {
        /// Mostra também as concluídas e as descartadas
        #[arg(short = 't', long = "todas")]
        all: bool,

        /// Só as tarefas dessa área
        #[arg(short = 'a', long = "area", value_name = "ÁREA")]
        area: Option<Area>,

        /// Só as tarefas nesse status
        #[arg(short = 's', long = "status", value_name = "STATUS")]
        status: Option<Status>,
    },

    /// Conclui uma tarefa: gol!
    #[command(
        override_usage = "vamo done <ID>",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        next_help_heading = "Opções"
    )]
    Done {
        /// O ID da tarefa, como T-041 ou só 41
        #[arg(value_name = "ID", value_parser = parse_task_id, help_heading = "Argumentos")]
        id: TaskId,
    },

    /// Mostra a ficha de uma tarefa, com o histórico
    #[command(
        override_usage = "vamo show <ID>",
        help_template = SUBCOMMAND_HELP_TEMPLATE,
        next_help_heading = "Opções"
    )]
    Show {
        /// O ID da tarefa, como T-041 ou só 41
        #[arg(value_name = "ID", value_parser = parse_task_id, help_heading = "Argumentos")]
        id: TaskId,
    },
}

/// Lê o ID de uma tarefa com um pouco de folga pra quem digita: além de
/// `T-041`, aceita `t-041` e o número sozinho (`41`).
fn parse_task_id(text: &str) -> Result<TaskId, vamo_core::Error> {
    let canonical = if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        format!("T-{text}")
    } else if let Some(number) = text.strip_prefix("t-") {
        format!("T-{number}")
    } else {
        text.to_owned()
    };
    canonical
        .parse()
        .map_err(|_| vamo_core::Error::InvalidTaskId {
            input: text.to_owned(),
        })
}

/// Traduz um erro de uso do clap, que vem em inglês, pra uma frase em
/// português.
pub(crate) fn usage_error(error: &clap::Error) -> String {
    let context = |kind| error.get(kind).map(ToString::to_string).unwrap_or_default();
    match error.kind() {
        ErrorKind::InvalidSubcommand => {
            let command = context(ContextKind::InvalidSubcommand);
            match error.get(ContextKind::SuggestedSubcommand) {
                Some(suggestion) => {
                    format!("o comando `{command}` não existe: não seria `{suggestion}`?")
                }
                None => format!("o comando `{command}` não existe"),
            }
        }
        ErrorKind::UnknownArgument => {
            format!("não entendi o `{}`", context(ContextKind::InvalidArg))
        }
        ErrorKind::MissingRequiredArgument => {
            format!("faltou {}", context(ContextKind::InvalidArg))
        }
        ErrorKind::ValueValidation => {
            // O erro de validação traz dentro a mensagem do domínio, que já
            // está em português e explica o formato certo.
            match std::error::Error::source(error) {
                Some(source) => source.to_string(),
                None => format!(
                    "o valor `{}` não serve pra {}",
                    context(ContextKind::InvalidValue),
                    context(ContextKind::InvalidArg)
                ),
            }
        }
        ErrorKind::InvalidValue => {
            let mut message = format!(
                "o valor `{}` não serve pra {}",
                context(ContextKind::InvalidValue),
                context(ContextKind::InvalidArg)
            );
            if let Some(valid) = error.get(ContextKind::ValidValue) {
                message = format!("{message}: os valores aceitos são {valid}");
            }
            message
        }
        ErrorKind::ArgumentConflict => format!(
            "{} não combina com {}",
            context(ContextKind::InvalidArg),
            context(ContextKind::PriorArg)
        ),
        ErrorKind::NoEquals | ErrorKind::TooManyValues | ErrorKind::TooFewValues => format!(
            "o valor de {} veio do jeito errado",
            context(ContextKind::InvalidArg)
        ),
        // O que sobra é raro o bastante pra ficar com a mensagem do clap.
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn task_ids_can_be_typed_loosely() {
        let id = TaskId::new(41).unwrap();
        for text in ["T-041", "T-41", "t-041", "41", "041"] {
            assert_eq!(parse_task_id(text), Ok(id), "{text:?}");
        }
    }

    #[test]
    fn a_bad_task_id_keeps_what_was_typed_in_the_error() {
        for text in ["", "T-0", "0", "x41", "-41"] {
            assert_eq!(
                parse_task_id(text),
                Err(vamo_core::Error::InvalidTaskId {
                    input: text.to_owned()
                })
            );
        }
    }
}
