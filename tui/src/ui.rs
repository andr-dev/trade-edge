use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Cell, Row, Table, Tabs},
};

use crate::{
    animation::RankDir,
    app::{App, ViewMode},
};

pub fn draw(frame: &mut Frame, app: &App) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(frame.area());

    draw_cumulative_chart(frame, vert[0], app);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(36),
            Constraint::Length(20),
            Constraint::Min(0),
        ])
        .split(vert[1]);

    draw_leaderboard(frame, bottom[0], app);
    draw_trade_log(frame, bottom[1], app);
    draw_pnl_bar_chart(frame, bottom[2], app);
}

fn draw_cumulative_chart(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};

    let border_style = Style::default().fg(Color::DarkGray);

    let active = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let inactive = Style::default().fg(Color::DarkGray);

    let selected = match app.view_mode {
        ViewMode::Cumulative => 0,
        ViewMode::Recent => 1,
    };
    let tabs = Tabs::new(vec!["Cumulative", "Recent"])
        .select(selected)
        .style(inactive)
        .highlight_style(active)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(border_style),
        );

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    frame.render_widget(tabs, chunks[0]);

    if app.users.is_empty() {
        return;
    }

    let (chart_lines, y_min, y_max) = match app.view_mode {
        ViewMode::Cumulative => {
            let max_len = app
                .users
                .values()
                .map(|s| s.cumulative_pnl.len())
                .max()
                .unwrap_or(1)
                .max(1);

            let mut y_min = 0.0_f64;
            let mut y_max = 1.0_f64;
            for stats in app.users.values() {
                for &v in &stats.cumulative_pnl {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                }
            }
            y_min = y_min.floor();
            y_max = y_max.ceil();

            let lines: Vec<(String, Color, Vec<f64>)> = app
                .users
                .iter()
                .map(|(name, stats)| {
                    let color = app
                        .color_map
                        .get(name.as_str())
                        .copied()
                        .unwrap_or(Color::White);
                    let mut pts = stats.cumulative_pnl.clone();
                    pts.resize(max_len, *pts.last().unwrap_or(&0.0));
                    (name.clone(), color, pts)
                })
                .collect();

            (lines, y_min, y_max)
        }
        ViewMode::Recent => {
            let n = app.users.len();
            let mut current: std::collections::HashMap<String, f64> =
                app.users.keys().map(|k| (k.clone(), 0.0)).collect();
            let mut series: std::collections::HashMap<String, Vec<f64>> =
                app.users.keys().map(|k| (k.clone(), Vec::new())).collect();

            for (user, profit) in &app.history {
                for (name, val) in &mut current {
                    if name == user {
                        *val += profit;
                    } else if n > 1 {
                        *val -= profit / (n - 1) as f64;
                    }
                }
                for (name, val) in &current {
                    series.get_mut(name).unwrap().push(*val);
                }
            }

            let mut y_min = 0.0_f64;
            let mut y_max = 1.0_f64;
            for pts in series.values() {
                for &v in pts {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                }
            }
            y_min = y_min.floor();
            y_max = y_max.ceil();

            let lines: Vec<(String, Color, Vec<f64>)> = series
                .into_iter()
                .map(|(name, pts)| {
                    let color = app.color_map.get(&name).copied().unwrap_or(Color::White);
                    (name, color, pts)
                })
                .collect();

            (lines, y_min, y_max)
        }
    };

    let mut chart_lines = chart_lines;
    chart_lines.sort_by(|a, b| {
        let a_last = a.2.last().copied().unwrap_or(0.0);
        let b_last = b.2.last().copied().unwrap_or(0.0);
        a_last.total_cmp(&b_last).then_with(|| a.0.cmp(&b.0))
    });

    let x_max = chart_lines
        .iter()
        .map(|(_, _, pts)| pts.len().saturating_sub(1))
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let canvas = Canvas::default()
        .x_bounds([0.0, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: x_max,
                y2: 0.0,
                color: Color::DarkGray,
            });

            for (_name, color, pts) in &chart_lines {
                for i in 1..pts.len() {
                    ctx.draw(&CanvasLine {
                        x1: i as f64 - 1.0,
                        y1: pts[i - 1],
                        x2: i as f64,
                        y2: pts[i],
                        color: *color,
                    });
                }
            }

            for y in [y_min, y_max] {
                ctx.print(
                    0.0,
                    y,
                    Span::styled(format!("${:.0}", y), Style::default().fg(Color::DarkGray)),
                );
            }
        });

    frame.render_widget(canvas, chunks[1]);
}

fn draw_trade_log(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = Style::default().fg(Color::DarkGray);
    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let widths = [Constraint::Length(12), Constraint::Length(6)];

    let outer = Block::default()
        .title(" Recent Trades")
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    let header_table = Table::new(
        vec![Row::new(vec![
            Cell::from("User").style(header_style),
            Cell::from(Text::from("Profit").alignment(Alignment::Right)).style(header_style),
        ])],
        widths,
    )
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(border_style),
    );
    frame.render_widget(header_table, chunks[0]);

    let rows: Vec<Row> = app
        .history
        .iter()
        .rev()
        .map(|(user, profit)| {
            let color = app.color_map.get(user).copied().unwrap_or(Color::White);
            let pnl_color = if *profit >= 0.0 {
                Color::Green
            } else {
                Color::Red
            };
            let sign = if *profit >= 0.0 { "+" } else { "" };
            Row::new(vec![
                Cell::from(user.as_str()).style(Style::default().fg(color)),
                Cell::from(Text::from(format!("{sign}{:.2}", profit)).alignment(Alignment::Right))
                    .style(Style::default().fg(pnl_color)),
            ])
        })
        .collect();

    frame.render_widget(Table::new(rows, widths), chunks[1]);
}

fn draw_pnl_bar_chart(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = Style::default().fg(Color::DarkGray);

    if app.users.is_empty() {
        let block = Block::default()
            .title(" PNL per User ")
            .borders(Borders::ALL)
            .border_style(border_style);
        frame.render_widget(block, area);
        return;
    }

    let mut entries: Vec<(&str, f64, Color)> = app
        .users
        .iter()
        .map(|(name, stats)| {
            let color = app
                .color_map
                .get(name.as_str())
                .copied()
                .unwrap_or(Color::White);
            (name.as_str(), stats.total_pnl, color)
        })
        .collect();
    entries.sort_by(|a, b| b.1.total_cmp(&a.1));

    let bars: Vec<Bar> = entries
        .iter()
        .map(|(name, pnl, color)| {
            let value = (pnl * 100.0).round().max(0.0) as u64;
            Bar::default()
                .label(name.to_string().into())
                .value(value)
                .text_value(String::new())
                .style(Style::default().fg(*color))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(" PNL Distribution ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(1);

    frame.render_widget(bar_chart, area);
}

fn flash_bg(intensity: f64) -> Color {
    let v = (intensity * 255.0) as u8;
    Color::Rgb(v, v, v)
}

fn draw_leaderboard(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = Style::default().fg(Color::DarkGray);
    let leaderboard = app.leaderboard();

    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let widths = [
        Constraint::Length(4),
        Constraint::Length(2),
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(6),
    ];

    let outer = Block::default()
        .title(" Leaderboard ")
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    let header_table = Table::new(
        vec![Row::new(vec![
            Cell::from("Rank").style(header_style),
            Cell::from("").style(header_style),
            Cell::from("User").style(header_style),
            Cell::from(Text::from("PNL ($)").alignment(Alignment::Right)).style(header_style),
            Cell::from(Text::from("Trades").alignment(Alignment::Right)).style(header_style),
        ])],
        widths,
    )
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(border_style),
    );
    frame.render_widget(header_table, chunks[0]);

    let rows: Vec<Row> = leaderboard
        .iter()
        .enumerate()
        .map(|(i, (name, stats))| {
            let color = app.color_map.get(*name).copied().unwrap_or(Color::White);
            let rank_style = match i {
                0 => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                1 => Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
                2 => Style::default().fg(Color::Rgb(205, 127, 50)),
                _ => Style::default().fg(Color::DarkGray),
            };

            let arrow_cell = match app.anim.rank_arrow(name) {
                Some((RankDir::Up, intensity)) => {
                    let g = (intensity * 255.0) as u8;
                    Cell::from("▲").style(Style::default().fg(Color::Rgb(0, g, 0)))
                }
                Some((RankDir::Down, intensity)) => {
                    let r = (intensity * 255.0) as u8;
                    Cell::from("▼").style(Style::default().fg(Color::Rgb(r, 0, 0)))
                }
                None => Cell::from(" "),
            };

            let pnl_color = if stats.total_pnl >= 0.0 {
                Color::Green
            } else {
                Color::Red
            };

            let flash = app.anim.flash_intensity(name);
            let row_style = if flash > 0.0 {
                Style::default().bg(flash_bg(flash))
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("#{}", i + 1)).style(rank_style),
                arrow_cell,
                Cell::from(name.to_string()).style(Style::default().fg(color)),
                Cell::from(
                    Text::from(format!("{:.2}", stats.total_pnl)).alignment(Alignment::Right),
                )
                .style(Style::default().fg(pnl_color)),
                Cell::from(Text::from(stats.trade_count.to_string()).alignment(Alignment::Right))
                    .style(Style::default().fg(Color::White)),
            ])
            .style(row_style)
        })
        .collect();

    frame.render_widget(Table::new(rows, widths), chunks[1]);
}
