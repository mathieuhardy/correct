use crate::ui::app::FileBuffer;

pub fn show(ui: &mut egui::Ui, file: Option<&mut FileBuffer>) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(buf) = file {
                let res = ui.add(
                    egui::TextEdit::multiline(&mut buf.content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .frame(egui::Frame::new()),
                );
                if res.changed() {
                    buf.modified = true;
                }
            }
        });
}
