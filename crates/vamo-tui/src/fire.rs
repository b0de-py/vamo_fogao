//! O fogo alvinegro: o fogo do Doom queimando atrás da tela inteira, igual
//! ao do r4ul.com.br, só que em preto e branco.
//!
//! A conta é a mesma do site (`js/common_effects.js`, que por sua vez é o
//! fogo do Doom do PlayStation):
//!
//! - o calor de cada célula vai de 0 a 36;
//! - as linhas de baixo são a brasa, acesa no máximo a cada passo;
//! - a cada passo, cada célula copia o calor da de baixo, perdendo de 0 a 3
//!   pontos ao acaso, e o vento joga o resultado uma célula pro lado;
//! - um estouro (no site, o clique; aqui, o gol e o clique) acende um
//!   círculo inteiro, que sobe e esfria como o resto.
//!
//! O que muda do site pra cá:
//!
//! - a cor é uma só, o branco (no site ela troca a cada 5 segundos);
//! - a altura da brasa acompanha o placar do dia. Com o placar fechado, ela
//!   ocupa os 18% de baixo da tela, como no site;
//! - o acaso vem de um gerador com semente fixa, então a mesma semente e os
//!   mesmos passos dão sempre o mesmo fogo, e dá pra testar.
//!
//! No site, o fogo é um canvas com 25% de opacidade atrás de caixas de
//! vidro fumê. Aqui, o [`Fire::underlay`] faz o mesmo papel: mistura o fogo
//! com o fundo e deixa o texto num vidro mais escuro, pra continuar legível.
//! Cada linha do terminal mostra duas linhas do fogo com o meio-bloco `▀`:
//! a cor da letra é a célula de cima e a do fundo, a de baixo.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::Span;

use crate::star::Star;
use crate::theme;

/// A semente do fogo do app. Qualquer número serve; fixo, o fogo começa
/// igual toda vez.
pub(crate) const SEED: u64 = 0xF060_1904_0812_0002;

/// O calor máximo: as 37 intensidades (0 a 36) do fogo do Doom.
const HOTTEST: u8 = 36;

/// A fatia de baixo da tela que a brasa ocupa com o placar fechado. É a
/// brasa do site.
const BASE: f64 = 0.18;

/// O quanto do fogo aparece por cima do fundo. É a opacidade do canvas do
/// site.
const OPACITY: f64 = 0.25;

/// O quanto do fogo passa pelo vidro que fica atrás do texto e das caixas.
/// No site, as caixas de vidro fumê deixam passar de 10% a 35%. Com um
/// quarto, até o texto apagado continua legível em cima da brasa.
const GLASS: f64 = 0.25;

/// O brilho da estrela fria, sem fogo passando por ela: de 0 (o fundo) a 1
/// (branco). O fogo esquenta a estrela daí até o branco.
const STARLIGHT: f64 = 0.45;

/// O raio de um estouro, em células do fogo.
const BLAST: usize = 7;

/// Até quantas células vazias seguidas, entre dois textos da mesma linha,
/// ainda ficam atrás do vidro. Sem isso, o espaço entre duas colunas de uma
/// tabela viraria uma fresta clara no meio da linha. Vinte células cobrem a
/// coluna mais larga do Banco e ainda deixam o fogo aparecer entre a lista
/// e o quadro da curiosidade.
const BRIDGE: usize = 20;

/// O fogo: uma grade de calor, de 0 (apagado) a [`HOTTEST`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fire {
    width: usize,
    /// Linhas do fogo, o dobro das linhas do terminal.
    height: usize,
    /// O calor de cada célula, linha por linha, de cima pra baixo.
    heat: Vec<u8>,
    /// O estado do gerador de acaso.
    state: u64,
}

impl Fire {
    /// Um fogo apagado, com a semente dada.
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            width: 0,
            height: 0,
            heat: Vec::new(),
            state: seed,
        }
    }

    /// Ajusta o tamanho do fogo ao da área, em células do terminal. Com o
    /// tamanho mudado, o fogo recomeça apagado.
    pub(crate) fn resize(&mut self, columns: u16, rows: u16) {
        let (width, height) = (usize::from(columns), usize::from(rows) * 2);
        if (width, height) != (self.width, self.height) {
            self.width = width;
            self.height = height;
            self.heat = vec![0; width * height];
        }
    }

    /// Avança um passo: acende a brasa e espalha o calor. A `strength`, de 0
    /// a 1, é o placar do dia (ou 1 no grito de gol) e dá a altura da brasa.
    pub(crate) fn step(&mut self, strength: f64) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        self.feed(strength);
        self.spread();
    }

    /// Acende a brasa: as linhas de baixo, no calor máximo. Com o placar
    /// zerado, sobra o fogo piloto, de uma linha só.
    fn feed(&mut self, strength: f64) {
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a altura de um terminal cabe com folga num f64, e a conta fica entre 0 e ela"
        )]
        let rows = (self.height as f64 * BASE * strength.clamp(0.0, 1.0)).round() as usize;
        let rows = rows.clamp(1, self.height);
        let first = (self.height - rows) * self.width;
        self.heat[first..].fill(HOTTEST);
    }

    /// Espalha o calor pra cima, do jeito do site: coluna por coluna, cada
    /// célula recebe o calor da de baixo, menos a perda, e o vento escreve o
    /// resultado na célula ao lado. A linha de baixo é só brasa.
    ///
    /// No site, a linha do topo fica de fora da conta, e um estouro lá em
    /// cima nunca mais apaga. Aqui ela entra.
    fn spread(&mut self) {
        let losses = self.losses();
        for x in 0..self.width {
            for y in 0..self.height - 1 {
                let index = y * self.width + x;
                let random = self.next();
                let loss = u8::try_from(random % losses).unwrap_or(0);
                // O vento: uma célula pra direita, nenhuma ou uma pra
                // esquerda. Na beirada, a chama dá a volta pra linha vizinha.
                let wind = usize::try_from((random >> 8) % 3).unwrap_or(1);
                let Some(target) = (index + 1).checked_sub(wind) else {
                    continue;
                };
                self.heat[target] = self.heat[index + self.width].saturating_sub(loss);
            }
        }
    }

    /// Quantas perdas diferentes uma célula pode sortear a cada passo.
    ///
    /// No site são 4: de 0 a 3 pontos, e a chama sobe umas 24 linhas do
    /// fogo acima da brasa. Num terminal de 36 linhas ou mais, é assim aqui
    /// também. Num terminal mais baixo, a mesma chama tomaria a tela toda,
    /// então a perda cresce pra chama ficar com um quarto da altura.
    fn losses(&self) -> u64 {
        /// A perda do site: de 0 a 3.
        const SITE: u64 = 3;
        /// A altura, em linhas do fogo, vezes a maior perda que mantém a
        /// chama num quarto dela. A chama sobe o calor máximo dividido pela
        /// perda média: 36 / (perda / 2) = altura / 4.
        const REACH: u64 = 2 * 4 * HOTTEST as u64;
        let height = u64::try_from(self.height).unwrap_or(u64::MAX).max(1);
        (REACH / height).max(SITE) + 1
    }

    /// Um estouro com o centro numa célula do terminal: acende um círculo
    /// no calor máximo. É o clique do site.
    pub(crate) fn explode(&mut self, column: u16, row: u16) {
        self.blast(usize::from(column), usize::from(row) * 2);
    }

    /// Um estouro num ponto sorteado da metade de cima do fogo, onde a chama
    /// não chega sozinha. É o foguete do gol.
    pub(crate) fn burst(&mut self) {
        if self.width == 0 || self.height < 4 {
            return;
        }
        let random = self.next();
        let (width, band) = (self.width as u64, (self.height / 2) as u64);
        let x = usize::try_from(random % width).unwrap_or(0);
        let y = usize::try_from((random >> 32) % band).unwrap_or(0) + self.height / 4;
        self.blast(x, y);
    }

    /// Acende o círculo de raio [`BLAST`] em volta de uma célula do fogo.
    fn blast(&mut self, center_x: usize, center_y: usize) {
        for y in center_y.saturating_sub(BLAST)..(center_y + BLAST + 1).min(self.height) {
            for x in center_x.saturating_sub(BLAST)..(center_x + BLAST + 1).min(self.width) {
                let (dx, dy) = (x.abs_diff(center_x), y.abs_diff(center_y));
                if dx * dx + dy * dy <= BLAST * BLAST {
                    self.heat[y * self.width + x] = HOTTEST;
                }
            }
        }
    }

    /// O calor de uma célula, de 0 a 36. Fora da grade, zero.
    fn heat_at(&self, x: usize, y: usize) -> u8 {
        if x < self.width && y < self.height {
            self.heat[y * self.width + x]
        } else {
            0
        }
    }

    /// Quantas linhas do fogo têm alguma chama.
    #[cfg(test)]
    fn flame_height(&self) -> usize {
        (0..self.height)
            .find(|&y| (0..self.width).any(|x| self.heat[y * self.width + x] > 0))
            .map_or(0, |top| self.height - top)
    }

    /// O próximo número do gerador (xorshift64*, curto e bom o bastante pra
    /// enfeite).
    fn next(&mut self) -> u64 {
        // O xorshift não sai do zero; a semente zero vira outra.
        if self.state == 0 {
            self.state = SEED;
        }
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// O brilho de uma célula do fogo, de 0 (o fundo da tela) a 1 (branco).
    ///
    /// Sem estrela, é o calor visto com a opacidade do site. Dentro da
    /// estrela, parte do brilho dela e sobe com o calor: a chama que passa
    /// esquenta a estrela até o branco.
    fn light(&self, star: Option<&Star>, x: usize, y: usize) -> f64 {
        let heat = f64::from(self.heat_at(x, y)) / f64::from(HOTTEST);
        let flame = heat * OPACITY;
        let Some(star) = star else { return flame };
        let covered = star.coverage(x, y);
        if covered == 0.0 {
            return flame;
        }
        let starlight = if star.is_glowing() {
            1.0
        } else {
            STARLIGHT + (1.0 - STARLIGHT) * heat
        };
        // Na borda, a célula é parte estrela e parte fogo.
        flame + (starlight - flame) * covered
    }

    /// Põe o fogo, e a estrela se tiver, por baixo do que já está desenhado
    /// em `area`.
    ///
    /// Roda depois de todas as telas, e olha célula por célula:
    ///
    /// - célula vazia no fundo da tela: mostra o fogo com a opacidade do
    ///   site;
    /// - célula com texto, vazia mas colada num texto, ou dentro de uma
    ///   caixa ([`theme::SURFACE`]): mostra o fogo atrás do vidro, mais
    ///   escuro, pra letra continuar legível;
    /// - célula com outro fundo (a linha do cursor, o grito de gol): fica
    ///   como está.
    pub(crate) fn underlay(&self, area: Rect, buf: &mut Buffer, star: Option<&Star>) {
        let columns = usize::from(area.width);
        for row in 0..usize::from(area.height) {
            let Ok(dy) = u16::try_from(row) else { continue };
            let y = area.y + dy;
            let behind_glass = behind_glass(&inks(buf, area.x, y, columns));
            for (x, &behind_glass) in behind_glass.iter().enumerate() {
                let top = self.light(star, x, row * 2);
                let bottom = self.light(star, x, row * 2 + 1);
                if top == 0.0 && bottom == 0.0 {
                    continue;
                }
                let Ok(dx) = u16::try_from(x) else { continue };
                let Some(cell) = buf.cell_mut((area.x + dx, y)) else {
                    continue;
                };
                let boxed = match cell.bg {
                    color if color == theme::BACKGROUND => false,
                    color if color == theme::SURFACE => true,
                    _ => continue,
                };
                if boxed || behind_glass {
                    // Atrás do vidro, a célula fica com o caractere que tem
                    // (assim, quem copia um texto da tela não leva
                    // meio-bloco no lugar dos espaços) e só o fundo muda. O
                    // fundo é uma cor só: a média das duas células do fogo
                    // que passam por ali.
                    cell.set_bg(shade(f64::midpoint(top, bottom) * GLASS));
                } else {
                    cell.set_char('▀').set_fg(shade(top)).set_bg(shade(bottom));
                    // O meio-bloco não herda negrito nem sublinhado de quem
                    // desenhou a linha.
                    cell.modifier = Modifier::empty();
                }
            }
        }
    }
}

/// O que uma célula do terminal tem desenhado, do ponto de vista do fogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ink {
    /// Nada: o fogo pode aparecer.
    Blank,
    /// Um traço de borda ou de separador. Fica atrás do vidro, mas não
    /// puxa as células vizinhas.
    Border,
    /// Texto, ou a segunda metade de um caractere largo, que não pode ser
    /// coberta.
    Text,
}

/// O que cada célula de uma linha do terminal tem desenhado.
fn inks(buf: &Buffer, left: u16, y: u16, columns: usize) -> Vec<Ink> {
    let mut inks = vec![Ink::Blank; columns];
    for x in 0..columns {
        let Ok(dx) = u16::try_from(x) else { continue };
        let Some(cell) = buf.cell((left + dx, y)) else {
            continue;
        };
        let symbol = cell.symbol();
        if symbol == " " {
            continue;
        }
        // Os traços de caixa do Unicode vão de U+2500 a U+257F.
        if symbol
            .chars()
            .all(|c| ('\u{2500}'..='\u{257F}').contains(&c))
        {
            inks[x] = Ink::Border;
        } else {
            let width = Span::raw(symbol).width().max(1);
            for covered in inks.iter_mut().skip(x).take(width) {
                *covered = Ink::Text;
            }
        }
    }
    inks
}

/// Quais células de uma linha ficam atrás do vidro: as que têm alguma coisa
/// desenhada, as coladas num texto e as que estão num vão de até [`BRIDGE`]
/// células entre dois textos.
fn behind_glass(inks: &[Ink]) -> Vec<bool> {
    // A distância de cada célula até o texto mais próximo, andando numa
    // direção só.
    fn distances(inks: impl Iterator<Item = Ink>) -> Vec<usize> {
        let mut distance = usize::MAX;
        inks.map(|ink| {
            distance = if ink == Ink::Text {
                0
            } else {
                distance.saturating_add(1)
            };
            distance
        })
        .collect()
    }
    let left = distances(inks.iter().copied());
    let mut right = distances(inks.iter().rev().copied());
    right.reverse();
    inks.iter()
        .zip(left.iter().zip(&right))
        .map(|(&ink, (&left, &right))| {
            // O vão entre dois textos tem `left + right - 1` células.
            let gap = left.saturating_add(right).saturating_sub(1);
            ink != Ink::Blank || left <= 1 || right <= 1 || gap <= BRIDGE
        })
        .collect()
}

/// A cor de um brilho, de 0 (o fundo quase preto) a 1 (branco), sempre com
/// R, G e B iguais. Vermelho não entra, nem no fogo.
fn shade(light: f64) -> Color {
    let backdrop = f64::from(theme::BACKGROUND_GRAY);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "o cinza fica entre o do fundo e 255"
    )]
    let gray = (backdrop + (255.0 - backdrop) * light.clamp(0.0, 1.0)).round() as u8;
    Color::Rgb(gray, gray, gray)
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::style::Style;

    use super::*;

    /// Um fogo de 24 colunas por 12 linhas do terminal, depois de `steps`
    /// passos.
    fn burnt(strength: f64, steps: usize) -> Fire {
        burnt_in(12, strength, steps)
    }

    /// Um fogo de 24 colunas por `rows` linhas do terminal, depois de
    /// `steps` passos. Com 40 linhas, a chama não chega no alto.
    fn burnt_in(rows: u16, strength: f64, steps: usize) -> Fire {
        let mut fire = Fire::new(SEED);
        fire.resize(24, rows);
        for _ in 0..steps {
            fire.step(strength);
        }
        fire
    }

    /// O fogo em texto, um caractere por célula: do `.` (apagado) ao `Z`
    /// (calor 36), em base 36.
    fn picture(fire: &Fire) -> String {
        (0..fire.height)
            .map(|y| {
                (0..fire.width)
                    .map(|x| match fire.heat_at(x, y) {
                        0 => '.',
                        heat => char::from_digit(u32::from(heat) - 1, 36)
                            .unwrap_or('?')
                            .to_ascii_uppercase(),
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Um terminal de mentira com o fundo do app, do tamanho do fogo.
    fn screen(fire: &Fire) -> (Buffer, Rect) {
        let area = Rect::new(
            0,
            0,
            u16::try_from(fire.width).unwrap(),
            u16::try_from(fire.height / 2).unwrap(),
        );
        let mut buf = Buffer::empty(area);
        buf.set_style(area, theme::base());
        (buf, area)
    }

    fn gray(color: Color) -> u8 {
        let Color::Rgb(r, g, b) = color else {
            panic!("o fogo só usa cinza: {color:?}");
        };
        assert!(r == g && g == b, "{color:?} não é cinza");
        r
    }

    #[test]
    fn the_same_seed_burns_the_same() {
        assert_eq!(burnt(0.5, 40), burnt(0.5, 40));
        let mut other = Fire::new(SEED + 1);
        other.resize(24, 12);
        for _ in 0..40 {
            other.step(0.5);
        }
        assert_ne!(burnt(0.5, 40), other);
    }

    #[test]
    fn a_fixed_seed_gives_a_fixed_fire() {
        assert_snapshot!(picture(&burnt(0.6, 40)));
    }

    #[test]
    fn the_embers_grow_with_the_score() {
        let embers = |strength| {
            let mut fire = burnt(strength, 0);
            fire.feed(strength);
            fire.heat
                .chunks(fire.width)
                .filter(|row| row.iter().all(|&heat| heat == HOTTEST))
                .count()
        };
        assert_eq!(embers(0.0), 1, "o fogo piloto apagou");
        assert_eq!(embers(0.5), 2);
        // 18% das 24 linhas do fogo, como no site.
        assert_eq!(embers(1.0), 4);
    }

    #[test]
    fn the_flame_grows_with_the_score() {
        // Num fogo alto o bastante pra chama não bater no teto, a média de
        // muitos passos, pra o acaso não decidir sozinho.
        let average = |strength| {
            let mut fire = Fire::new(SEED);
            fire.resize(24, 40);
            let mut total = 0;
            for step in 0..300 {
                fire.step(strength);
                if step >= 100 {
                    total += fire.flame_height();
                }
            }
            total / 200
        };
        let (empty, half, full) = (average(0.0), average(0.5), average(1.0));
        assert!(empty < half && half < full, "{empty} {half} {full}");
        assert!(empty > 0, "o fogo piloto apagou");
    }

    #[test]
    fn a_new_size_starts_cold() {
        let mut fire = burnt(1.0, 10);
        fire.resize(30, 2);
        assert_eq!(fire.flame_height(), 0);
        // O mesmo tamanho não apaga nada.
        fire.step(1.0);
        fire.resize(30, 2);
        assert!(fire.flame_height() > 0);
    }

    #[test]
    fn an_explosion_lights_a_circle_that_rises_and_cools() {
        let mut fire = burnt_in(40, 0.0, 0);
        fire.explode(12, 6);
        assert_eq!(fire.heat_at(12, 12), HOTTEST);
        assert_eq!(fire.heat_at(12 + BLAST, 12), HOTTEST);
        assert_eq!(fire.heat_at(12 + BLAST + 1, 12), 0);
        // Na beirada, o círculo é cortado sem estourar a grade.
        fire.explode(0, 0);
        fire.explode(23, 11);
        // Sem brasa nova embaixo dele, o círculo sobe e esfria até sumir.
        let lit = |fire: &Fire| fire.heat[..30 * fire.width].iter().any(|&heat| heat > 0);
        assert!(lit(&fire));
        for _ in 0..60 {
            fire.step(0.0);
        }
        assert!(!lit(&fire));
    }

    #[test]
    fn a_burst_stays_inside_the_fire() {
        let mut fire = burnt(0.0, 0);
        for _ in 0..50 {
            fire.burst();
        }
        assert!(fire.heat.contains(&HOTTEST));
        // Um fogo sem tamanho não tem onde estourar.
        Fire::new(SEED).burst();
    }

    #[test]
    fn the_fire_shows_dimmed_behind_the_empty_screen() {
        let fire = burnt_in(40, 1.0, 40);
        let (mut buf, area) = screen(&fire);
        fire.underlay(area, &mut buf, None);
        // A linha de baixo é brasa pura: o branco com 25% de opacidade.
        let ember = &buf[(5, area.height - 1)];
        assert_eq!(ember.symbol(), "▀");
        assert_eq!(gray(ember.bg), 71);
        // No alto, onde a chama não chega, a tela fica como estava.
        assert_eq!(buf[(0, 0)].symbol(), " ");
        assert_eq!(buf[(0, 0)].bg, theme::BACKGROUND);
    }

    #[test]
    fn text_stays_readable_behind_the_glass() {
        let fire = burnt(1.0, 40);
        let (mut buf, area) = screen(&fire);
        let last = area.height - 1;
        buf.set_string(2, last, "gol  de  placa", theme::muted());
        fire.underlay(area, &mut buf, None);
        // A letra continua lá, com o fogo bem mais escuro por trás.
        assert_eq!(buf[(2, last)].symbol(), "g");
        assert_eq!(buf[(2, last)].fg, theme::MUTED);
        assert!(gray(buf[(2, last)].bg) <= 25);
        // O vão entre as palavras e a célula colada no texto ficam atrás do
        // mesmo vidro, e continuam sendo espaço: quem copia o texto da tela
        // não leva meio-bloco junto. Longe do texto, o fogo aparece inteiro.
        for x in [1, 5, 6, 16] {
            assert_eq!(buf[(x, last)].symbol(), " ");
            assert!(gray(buf[(x, last)].bg) <= 25, "coluna {x}");
        }
        assert_eq!(buf[(20, last)].symbol(), "▀");
        assert_eq!(gray(buf[(20, last)].bg), 71);
    }

    #[test]
    fn a_box_is_glass_and_a_highlight_is_left_alone() {
        let fire = burnt(1.0, 40);
        let (mut buf, area) = screen(&fire);
        let last = area.height - 1;
        buf.set_style(Rect::new(0, last, 4, 1), Style::new().bg(theme::SURFACE));
        buf.set_style(Rect::new(10, last, 4, 1), theme::cursor());
        fire.underlay(area, &mut buf, None);
        assert_eq!(buf[(1, last)].symbol(), " ");
        assert!(gray(buf[(1, last)].bg) <= 25);
        assert_eq!(buf[(11, last)].symbol(), " ");
        assert_eq!(buf[(11, last)].bg, theme::ACCENT);
    }

    #[test]
    fn a_wide_character_keeps_its_second_half() {
        let fire = burnt(1.0, 40);
        let (mut buf, area) = screen(&fire);
        let last = area.height - 1;
        buf.set_string(4, last, "火", theme::text());
        fire.underlay(area, &mut buf, None);
        assert_eq!(buf[(4, last)].symbol(), "火");
        assert_eq!(buf[(5, last)].symbol(), " ");
    }

    #[test]
    fn the_star_shines_cold_and_the_fire_heats_it_to_white() {
        // Um fogo apagado, do tamanho de um terminal de 100 por 30, e a cor
        // do miolo da estrela (coluna 87, linha 20) depois de um quadro.
        let cold_fire = || {
            let mut fire = Fire::new(SEED);
            fire.resize(100, 30);
            fire
        };
        let heart = |fire: &Fire, star: &Star| {
            let (mut buf, area) = screen(fire);
            fire.underlay(area, &mut buf, Some(star));
            // Fora da estrela, sem fogo, a tela fica como estava.
            assert_eq!(buf[(5, 5)].symbol(), " ");
            assert_eq!(buf[(87, 20)].symbol(), "▀");
            gray(buf[(87, 20)].fg)
        };
        let star = Star::in_corner(100, 30).unwrap();
        assert!(star.coverage(87, 40) > 0.99);

        // Fria, a estrela tem o brilho dela.
        assert_eq!(heart(&cold_fire(), &star), 120);
        // Com a chama passando, ela esquenta até o branco.
        let mut fire = cold_fire();
        fire.explode(87, 20);
        assert_eq!(heart(&fire, &star), 255);
        // No grito de gol, ela acende inteira mesmo sem fogo.
        assert_eq!(heart(&cold_fire(), &star.glowing(true)), 255);
    }

    #[test]
    fn text_over_the_star_stays_readable() {
        let mut fire = Fire::new(SEED);
        fire.resize(100, 30);
        let (mut buf, area) = screen(&fire);
        buf.set_string(86, 20, "gol", theme::muted());
        fire.underlay(area, &mut buf, Some(&Star::in_corner(100, 30).unwrap()));
        assert_eq!(buf[(87, 20)].symbol(), "o");
        assert!(gray(buf[(87, 20)].bg) < 45);
    }

    #[test]
    fn no_red_in_the_fire() {
        for step in 0..=100 {
            gray(shade(f64::from(step) / 100.0));
        }
    }
}
