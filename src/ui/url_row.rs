use crate::storage::UrlRecord;
use crate::ui::ViewAction;

pub fn format_relative_timestamp(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let diff = now.saturating_sub(ts);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} min ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hr ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

pub fn url_row(
    ui: &mut egui::Ui,
    record: &UrlRecord,
    selected: bool,
    show_actions: bool,
) -> Option<ViewAction> {
    let mut action = None;

    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().panel_fill
    };

    let response = ui.scope(|ui| {
        ui.visuals_mut().override_text_color = if selected {
            Some(ui.visuals().selection.stroke.color)
        } else {
            None
        };

        egui::Frame::new()
            .fill(bg)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let title = record
                            .title
                            .as_deref()
                            .unwrap_or(&record.canonical_url);
                        ui.label(egui::RichText::new(title).strong());
                        ui.label(
                            egui::RichText::new(&record.canonical_url)
                                .size(12.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format_relative_timestamp(record.created_at))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );

                        if show_actions {
                            if ui.button("A").on_hover_text("Archive").clicked() {
                                action = Some(ViewAction::Archive(record.id));
                            }
                            if ui.button("P").on_hover_text("Pin").clicked() {
                                action = Some(ViewAction::Pin(record.id));
                            }
                            if ui.button("L").on_hover_text("Launch").clicked() {
                                action = Some(ViewAction::Launch(record.id));
                            }
                            if ui.button("D").on_hover_text("Delete").clicked() {
                                action = Some(ViewAction::Delete(record.id));
                            }
                        }
                    });
                });
            });
    });

    // Treat hover or selection as "show actions"
    let _hovered = response.response.hovered();

    action
}

#[cfg(test)]
mod tests {
    use super::format_relative_timestamp;

    #[test]
    fn timestamp_just_now() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now), "just now");
    }

    #[test]
    fn timestamp_two_minutes_ago() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 120), "2 min ago");
    }
}
