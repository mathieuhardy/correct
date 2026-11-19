use crate::ui::app::FileBuffer;

pub fn show(
    ui: &mut egui::Ui,
    file: Option<&mut FileBuffer>,
    line_height: f32,
    max_width: Option<f32>,
) {
    let Some(buf) = file else { return };

    let available = ui.available_width();
    let content_w = max_width.map(|mw| mw.min(available)).unwrap_or(available);
    let top_pad   = if max_width.is_some() { 24.0 } else { 0.0 };

    // Custom layouter only when line_height deviates from 1.0.
    // fonts_mut() takes a write lock — avoid calling it every frame unnecessarily.
    let custom_lh = (line_height - 1.0).abs() > 0.01;

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
                    ui.add(
                        egui::TextEdit::multiline(&mut buf.content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(content_w)
                            .frame(egui::Frame::new())
                            .interactive(false)
                            .layouter(&mut layouter),
                    )
                } else {
                    ui.add(
                        egui::TextEdit::multiline(&mut buf.content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(content_w)
                            .frame(egui::Frame::new())
                            .interactive(false),
                    )
                };
                if res.changed() {
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
}
