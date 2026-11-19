use crate::ui::app::FileBuffer;

pub fn show(
    ui: &mut egui::Ui,
    file: Option<&mut FileBuffer>,
    line_height: f32,
    max_width: Option<f32>,
) {
    let Some(buf) = file else { return };

    let font_size  = ui.text_style_height(&egui::TextStyle::Monospace);
    let text_color = ui.visuals().text_color();
    let lh_px      = font_size * line_height;
    let available  = ui.available_width();
    let content_w  = max_width.map(|mw| mw.min(available)).unwrap_or(available);
    let top_pad    = if max_width.is_some() { 24.0 } else { 0.0 };

    // Layouter custom : interligne configurable, héritage des couleurs et de la police.
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;
        job.append(
            text.as_str(),
            0.0,
            egui::TextFormat {
                font_id:     egui::FontId::monospace(font_size),
                line_height: Some(lh_px),
                color:       text_color,
                ..Default::default()
            },
        );
        ui.ctx().fonts_mut(|f| f.layout_job(job))
    };

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(top_pad);
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.set_max_width(content_w);
                let res = ui.add(
                    egui::TextEdit::multiline(&mut buf.content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(content_w)
                        .frame(egui::Frame::new())
                        .layouter(&mut layouter),
                );
                if res.changed() {
                    buf.modified = true;
                }
            });
        });
}
