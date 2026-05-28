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
        let days = diff / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
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

    let id = ui.id().with(record.id);
    let was_hovered: bool = ui
        .ctx()
        .memory(|mem| mem.data.get_temp(id).unwrap_or(false));
    let show = show_actions || was_hovered;

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
                        ui.add(egui::Label::new(egui::RichText::new(title).strong()).truncate());
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&record.canonical_url)
                                    .size(12.0)
                                    .color(ui.visuals().weak_text_color()),
                            )
                            .truncate(),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format_relative_timestamp(record.created_at))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );

                        if show {
                            if ui.button("D").on_hover_text("Delete").clicked() {
                                action = Some(ViewAction::Delete(record.id));
                            }
                            if ui.button("L").on_hover_text("Launch").clicked() {
                                action = Some(ViewAction::Launch(record.id));
                            }
                            if ui.button("P").on_hover_text("Pin").clicked() {
                                action = Some(ViewAction::Pin(record.id));
                            }
                            if ui.button("A").on_hover_text("Archive").clicked() {
                                action = Some(ViewAction::Archive(record.id));
                            }
                        }
                    });
                });
            });
    });

    let is_hovered = response.response.hovered();
    ui.ctx()
        .memory_mut(|mem| mem.data.insert_temp(id, is_hovered));

    if response.response.double_clicked() {
        action = Some(ViewAction::Launch(record.id));
    }

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

    #[test]
    fn timestamp_one_hour_ago() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 3600), "1 hr ago");
    }

    #[test]
    fn timestamp_one_day_ago() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 86400), "1 day ago");
    }

    #[test]
    fn timestamp_two_days_ago() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 86400 * 2), "2 days ago");
    }

    #[test]
    fn timestamp_one_day_ago_plus_one_second() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_timestamp(now - 86401), "1 day ago");
    }
}
