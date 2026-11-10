use crate::{
    data::{self, Grade, Lesson},
    theme::{Accent, MUTED, RED, TEXT},
    views::{self, ViewState},
};
use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::{BTreeMap, BTreeSet};

fn text(ui: &egui::Ui, rect: Rect, value: &str, size: f32, color: Color32) {
    if !rect.is_positive() {
        return;
    }
    let mut job = egui::text::LayoutJob::simple(
        value.to_owned(),
        FontId::proportional(size),
        color,
        rect.width(),
    );
    job.wrap.max_rows = 1;
    let galley = ui.painter().layout_job(job);
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(rect.min, galley, color);
}
fn line(rect: Rect, y: f32, h: f32) -> Rect {
    Rect::from_min_size(
        rect.min + Vec2::new(8., y),
        Vec2::new((rect.width() - 16.).max(1.), h),
    )
}
fn remaining(ui: &egui::Ui) -> f32 {
    (ui.clip_rect().bottom() - ui.cursor().top() - 10.).max(80.)
}

pub fn grades(
    ui: &mut egui::Ui,
    grouped: &BTreeMap<&str, Vec<&Grade>>,
    accent: Accent,
    state: &mut ViewState,
) {
    let columns = if ui.available_width() >= 940. {
        3
    } else if ui.available_width() >= 550. {
        2
    } else {
        1
    };
    let rows = ((remaining(ui) - 38.) / 108.).floor().max(1.) as usize;
    let per_page = rows * columns;
    let pages = grouped.len().div_ceil(per_page).max(1);
    state.grades_page = state.grades_page.min(pages - 1);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                state.grades_page > 0,
                egui::Button::new("‹").min_size(Vec2::new(28., 24.)),
            )
            .clicked()
        {
            state.grades_page -= 1;
        }
        ui.small(format!(
            "Strona {} / {} · szczegóły po kliknięciu",
            state.grades_page + 1,
            pages
        ));
        if ui
            .add_enabled(
                state.grades_page + 1 < pages,
                egui::Button::new("›").min_size(Vec2::new(28., 24.)),
            )
            .clicked()
        {
            state.grades_page += 1;
        }
    });
    let width = (ui.available_width() - 10. * (columns - 1) as f32) / columns as f32;
    let start = ui.cursor().min;
    let shown: Vec<_> = grouped
        .iter()
        .skip(state.grades_page * per_page)
        .take(per_page)
        .collect();
    let height = (remaining(ui) / rows as f32 - 8.).clamp(64., 100.);
    for (index, (subject, values)) in shown.iter().enumerate() {
        let rect = Rect::from_min_size(
            start
                + Vec2::new(
                    (index % columns) as f32 * (width + 10.),
                    (index / columns) as f32 * (height + 8.),
                ),
            Vec2::new(width, height),
        );
        ui.painter().rect(
            rect,
            6.,
            Color32::from_rgb(25, 32, 44),
            Stroke::new(1_f32, Color32::from_rgb(56, 69, 88)),
        );
        let response = ui.interact(rect, ui.id().with(("subject", subject)), Sense::click());
        response.clone().on_hover_text(format!(
            "{subject}\n{} ocen · kliknij, aby otworzyć szczegóły",
            values.len()
        ));
        if response.clicked() {
            state.grade_details = Some(subject.to_string());
        }
        let average = views::average_pair(values.iter().copied(), state.semester);
        let average_width = 126_f32.min(width * 0.52);
        text(
            ui,
            Rect::from_min_max(
                rect.min + Vec2::new(8., 8.),
                Pos2::new(rect.right() - average_width, rect.top() + 29.),
            ),
            subject,
            15.,
            TEXT,
        );
        text(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.right() - average_width, rect.top() + 8.),
                Vec2::new(average_width - 6., 22.),
            ),
            &average,
            14.,
            accent.color(),
        );
        let cols = ((width - 16.) / 37.).floor().max(1.) as usize;
        let badge_rows = ((height - 34.) / 29.).floor().max(1.) as usize;
        let capacity = cols * badge_rows;
        let visible = if values.len() > capacity {
            capacity.saturating_sub(1)
        } else {
            values.len()
        };
        for (i, g) in values.iter().take(visible).enumerate() {
            let badge = Rect::from_min_size(
                rect.min + Vec2::new(8. + (i % cols) as f32 * 37., 34. + (i / cols) as f32 * 29.),
                Vec2::new(32., 25.),
            );
            let color = views::grade_color(&g.value, accent);
            ui.painter().rect(
                badge,
                4.,
                color.gamma_multiply(0.14),
                Stroke::new(1_f32, color.gamma_multiply(0.4)),
            );
            if crate::shine::eligible(&g.value) {
                let shape = ui.painter().add(egui::Shape::Noop);
                crate::shine::paint(
                    ui,
                    shape,
                    badge,
                    ui.id().with((subject, i, &g.date)).value(),
                );
            }
            ui.painter().text(
                badge.center(),
                egui::Align2::CENTER_CENTER,
                &g.value,
                FontId::proportional(17.),
                color,
            );
            ui.interact(badge, ui.id().with((subject, i)), Sense::hover())
                .on_hover_text(format!(
                    "{}\n{} · {}\nWaga: {}",
                    g.category,
                    g.date,
                    g.teacher,
                    g.weight
                        .map(|w| w.to_string())
                        .unwrap_or_else(|| "—".into())
                ));
        }
        if visible < values.len() {
            let i = visible;
            text(
                ui,
                Rect::from_min_size(
                    rect.min
                        + Vec2::new(8. + (i % cols) as f32 * 37., 36. + (i / cols) as f32 * 29.),
                    Vec2::new(35., 23.),
                ),
                &format!("+{}", values.len() - visible),
                13.,
                MUTED,
            );
        }
    }
    ui.allocate_space(Vec2::new(
        ui.available_width(),
        shown.len().div_ceil(columns) as f32 * (height + 8.),
    ));
    if let Some(subject) = state.grade_details.clone() {
        let mut open = true;
        egui::Window::new(&subject)
            .id(egui::Id::new("grade_details"))
            .open(&mut open)
            .default_width(530.)
            .max_height(440.)
            .vscroll(true)
            .show(ui.ctx(), |ui| {
                if let Some(values) = grouped.get(subject.as_str()) {
                    ui.label(format!(
                        "Średnia arytmetyczna / ważona: {}",
                        views::average_pair(values.iter().copied(), state.semester)
                    ));
                    for g in values {
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(views::grade_color(&g.value, accent), &g.value);
                            ui.label(&g.category);
                            ui.small(format!(
                                "{} · {} · semestr {} · waga {}",
                                g.date,
                                g.teacher,
                                g.semester,
                                g.weight
                                    .map(|w| w.to_string())
                                    .unwrap_or_else(|| "—".into())
                            ));
                        });
                    }
                }
            });
        if !open {
            state.grade_details = None;
        }
    }
}

fn slot(lesson: &Lesson) -> (u32, String) {
    (
        lesson.number.parse().unwrap_or(u32::MAX),
        if lesson.number.parse::<u32>().is_ok() {
            String::new()
        } else {
            lesson.from.clone()
        },
    )
}
pub fn timetable(ui: &mut egui::Ui, items: &[Lesson], accent: Accent, week: NaiveDate) {
    let days = if items.iter().any(|l| {
        NaiveDate::parse_from_str(&l.date, "%Y-%m-%d")
            .is_ok_and(|d| d.weekday().num_days_from_monday() >= 5)
    }) {
        7
    } else {
        5
    };
    let slots: BTreeSet<_> = items
        .iter()
        .filter(|l| {
            NaiveDate::parse_from_str(&l.date, "%Y-%m-%d")
                .is_ok_and(|d| d >= week && d < week + Duration::days(days))
        })
        .map(slot)
        .collect();
    let gap = 18.;
    let width = (ui.available_width() - gap * (days - 1) as f32) / days as f32;
    let height = remaining(ui);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let row_height = (height - 76.) / slots.len().max(1) as f32;
    for day in 0..days {
        let date = week + Duration::days(day);
        let lessons: Vec<_> = items
            .iter()
            .filter(|l| l.date == date.to_string())
            .collect();
        let free = lessons.iter().all(|l| l.canceled);
        let rect = Rect::from_min_size(
            area.min + Vec2::new(day as f32 * (width + gap), 0.),
            Vec2::new(width, height),
        );
        let fill = if free {
            Color32::from_rgb(64, 25, 34)
        } else if day % 2 == 0 {
            Color32::from_rgb(20, 28, 40)
        } else {
            Color32::from_rgb(26, 35, 49)
        };
        let border = if free {
            RED
        } else if date == data::today() {
            accent.color()
        } else {
            Color32::from_rgb(68, 86, 111)
        };
        ui.painter()
            .rect(rect, 7., fill, Stroke::new(1.5_f32, border));
        let names = if width < 150. {
            ["PON.", "WT.", "ŚR.", "CZW.", "PT.", "SOB.", "ND."]
        } else {
            [
                "PONIEDZIAŁEK",
                "WTOREK",
                "ŚRODA",
                "CZWARTEK",
                "PIĄTEK",
                "SOBOTA",
                "NIEDZIELA",
            ]
        };
        text(ui, line(rect, 9., 18.), names[day as usize], 12., border);
        text(
            ui,
            line(rect, 29., 27.),
            &date.format("%d.%m").to_string(),
            22.,
            if free { RED } else { TEXT },
        );
        if free {
            text(ui, line(rect, 56., 16.), "Dzień wolny", 11., RED);
        }
        for (row, key) in slots.iter().enumerate() {
            let parallel: Vec<_> = lessons.iter().filter(|l| slot(l) == *key).collect();
            for (group, l) in parallel.iter().enumerate() {
                let cell_width = (width - 12.) / parallel.len() as f32;
                let card = Rect::from_min_size(
                    rect.min
                        + Vec2::new(
                            6. + group as f32 * cell_width,
                            76. + row as f32 * row_height,
                        ),
                    Vec2::new(cell_width - 3., (row_height - 5.).max(1.)),
                );
                let (fill, color, status) = if l.canceled {
                    (Color32::from_rgb(115, 36, 48), RED, "ODWOŁANA")
                } else if l.substitution {
                    (
                        Color32::from_rgb(133, 70, 25),
                        Color32::from_rgb(255, 196, 119),
                        "ZASTĘPSTWO",
                    )
                } else {
                    (Color32::from_rgb(32, 43, 59), accent.color(), "")
                };
                ui.painter().rect(
                    card,
                    4.,
                    fill,
                    Stroke::new(1_f32, color.gamma_multiply(0.55)),
                );
                let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(card));
                row_ui.set_clip_rect(card.intersect(ui.clip_rect()));
                text(
                    &row_ui,
                    line(card, 4., 15.),
                    &format!("{} · {}–{}", l.number, l.from, l.to),
                    10.,
                    color,
                );
                text(
                    &row_ui,
                    line(card, 21., 19.),
                    &l.subject,
                    if width < 150. { 12. } else { 14. },
                    TEXT,
                );
                if card.height() > 58. {
                    text(
                        &row_ui,
                        line(card, 42., 16.),
                        &if status.is_empty() {
                            format!("Sala {}", l.room)
                        } else {
                            status.to_string()
                        },
                        11.,
                        if status.is_empty() { MUTED } else { color },
                    );
                }
                if card.height() > 80. {
                    text(&row_ui, line(card, 62., 16.), &l.teacher, 11., MUTED);
                }
                ui.interact(
                    card,
                    ui.id().with(("lesson", day, row, group)),
                    Sense::hover(),
                )
                .on_hover_text(format!(
                    "{} · {}–{}\n{}\n{} · sala {}\n{}\n{}",
                    l.number, l.from, l.to, l.subject, l.teacher, l.room, status, l.note
                ));
            }
        }
    }
}
