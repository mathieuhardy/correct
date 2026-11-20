use crate::languagetool::LtMatch;
use crate::ui::app::FileBuffer;

pub fn show(
    ui: &mut egui::Ui,
    file: Option<&mut FileBuffer>,
    matches: &[LtMatch],
    line_height: f32,
    selected: Option<usize>,
    max_width: Option<f32>,
) -> Option<usize> {
    let buf = file?;

    let available = ui.available_width();
    let content_w = max_width.map(|mw| mw.min(available)).unwrap_or(available);
    let top_pad   = if max_width.is_some() { 24.0 } else { 0.0 };

    // Custom layouter only when line_height deviates from 1.0.
    // fonts_mut() takes a write lock — avoid calling it every frame unnecessarily.
    let custom_lh = (line_height - 1.0).abs() > 0.01;

    let mut clicked = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(top_pad);

            let mut add_editor = |ui: &mut egui::Ui| {
                let res = if custom_lh {
                    let font_size  = ui.text_style_height(&egui::TextStyle::Monospace);
                    let text_color = ui.visuals().text_color();
                    let lh_px      = font_size * line_height;
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap;
                        job.append(text.as_str(), 0.0, egui::TextFormat {
                            font_id:     egui::FontId::monospace(font_size),
                            line_height: Some(lh_px),
                            color:       text_color,
                            ..Default::default()
                        });
                        ui.ctx().fonts_mut(|f| f.layout_job(job))
                    };
                    egui::TextEdit::multiline(&mut buf.content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(content_w)
                        .frame(egui::Frame::new())
                        .interactive(false)
                        .layouter(&mut layouter)
                        .show(ui)
                } else {
                    egui::TextEdit::multiline(&mut buf.content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(content_w)
                        .frame(egui::Frame::new())
                        .interactive(false)
                        .show(ui)
                };
                paint_underlines(ui.painter(), &res.galley, res.galley_pos, matches, selected);

                // TextEdit non interactif ne capte que le survol : zone cliquable superposée.
                let click = ui.interact(res.response.rect, res.response.id.with("click"), egui::Sense::click());
                if let Some(pos) = click.hover_pos() {
                    let hit = match_at(&res.galley, res.galley_pos, matches, pos);
                    if hit.is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if click.clicked() {
                        clicked = hit;
                    }
                }
                if res.response.changed() {
                    buf.modified = true;
                }
            };

            if max_width.is_some() {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.set_max_width(content_w);
                    add_editor(ui);
                });
            } else {
                add_editor(ui);
            }
        });

    clicked
}

/// Rectangles (un par ligne visuelle) couverts par une erreur, relatifs au galley,
/// avec l'ordonnée de la ligne de base. Les offsets LT sont en caractères ; une erreur
/// coupée par le retour à la ligne produit plusieurs segments.
fn segments(galley: &egui::Galley, m: &LtMatch) -> Vec<(egui::Rect, f32)> {
    let (start, end) = (m.offset, m.offset + m.length);
    let mut out = Vec::new();
    let mut row_start = 0usize;

    for row in &galley.rows {
        if row_start >= end {
            break;
        }
        let (a, b) = (start.max(row_start), end.min(row_start + row.glyphs.len()));
        if a < b {
            let first = &row.glyphs[a - row_start];
            let last  = &row.glyphs[b - 1 - row_start];
            let rect = egui::Rect::from_min_max(
                egui::pos2(row.pos.x + first.pos.x, row.pos.y),
                egui::pos2(row.pos.x + last.pos.x + last.advance_width, row.pos.y + row.size.y),
            );
            out.push((rect, row.pos.y + first.pos.y));
        }
        row_start += row.char_count_including_newline().0;
    }
    out
}

fn visible(matches: &[LtMatch]) -> impl Iterator<Item = (usize, &LtMatch)> {
    matches.iter().enumerate().filter(|(_, m)| !m.hidden() && m.length > 0)
}

/// Souligne en rouge chaque erreur visible ; l'erreur sélectionnée est plus épaisse.
fn paint_underlines(
    painter: &egui::Painter,
    galley: &egui::Galley,
    origin: egui::Pos2,
    matches: &[LtMatch],
    selected: Option<usize>,
) {
    let color = egui::Color32::from_rgb(220, 80, 80);

    for (i, m) in visible(matches) {
        let width = if selected == Some(i) { 2.5 } else { 1.5 };
        for (rect, baseline) in segments(galley, m) {
            let y = origin.y + baseline + 4.0;
            painter.line_segment(
                [egui::pos2(origin.x + rect.min.x, y), egui::pos2(origin.x + rect.max.x, y)],
                egui::Stroke::new(width, color),
            );
        }
    }
}

/// Index de l'erreur sous `pos` (coordonnées écran), s'il y en a une.
fn match_at(galley: &egui::Galley, origin: egui::Pos2, matches: &[LtMatch], pos: egui::Pos2) -> Option<usize> {
    let rel = pos - origin.to_vec2();
    visible(matches)
        .find(|(_, m)| segments(galley, m).iter().any(|(r, _)| r.contains(rel)))
        .map(|(i, _)| i)
}
