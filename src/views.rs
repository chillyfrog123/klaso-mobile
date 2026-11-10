use crate::{
    data::{self, Attendance, Grade, Lesson, Message, Notice, Payload, Section},
    theme::{self, Accent, GREEN, MUTED, RED, TEXT},
};
#[cfg(test)]
use chrono::Duration;
use chrono::{Datelike, NaiveDate};
use eframe::egui::{self, RichText, Vec2};
use std::collections::{BTreeMap, BTreeSet};

pub struct ViewState {
    pub search: String,
    pub semester: usize,
    pub attendance_types: [bool; 5],
    pub grades_page: usize,
    pub grade_details: Option<String>,
}
impl Default for ViewState {
    fn default() -> Self {
        Self {
            search: String::new(),
            semester: 0,
            attendance_types: [true; 5],
            grades_page: 0,
            grade_details: None,
        }
    }
}
pub fn heading(
    ui: &mut egui::Ui,
    section: Section,
    accent: Accent,
    updated: Option<&str>,
    demo: bool,
) -> egui::Response {
    let today = data::today();
    let year = if today.month() >= 9 {
        today.year()
    } else {
        today.year() - 1
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(section.title()).size(22.).strong())
            .on_hover_text(section.subtitle());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if demo {
                theme::badge(ui, "PODGLĄD", accent.color());
            }
            let label = updated
                .map(|date| format!("Zapis: {date}"))
                .unwrap_or_else(|| today.format("%d.%m.%Y").to_string());
            ui.small(RichText::new(label).color(MUTED))
                .on_hover_text(if updated.is_some() {
                    "Ostatni udany zapis danych"
                } else {
                    "Dzisiejsza data"
                });
            if ui.available_width() > 260. {
                ui.small(RichText::new(format!("Rok szkolny {year}/{}", year + 1)).color(MUTED));
            }
        });
    })
    .response
}
fn dim(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).color(MUTED));
}
fn empty(ui: &mut egui::Ui, accent: Accent, title: &str, body: &str) {
    theme::glass(accent).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.add_space(22.);
        ui.label(RichText::new(title).size(21.).strong());
        dim(ui, body);
        ui.add_space(22.);
    });
}
fn stats(ui: &mut egui::Ui, accent: Accent, items: [(&str, String); 3]) {
    theme::glass(accent).inner_margin(egui::Margin::symmetric(16.,8.)).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.columns(3, |cols| {
            for (i, (label, value)) in items.into_iter().enumerate() {
                cols[i].with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(RichText::new(label).size(12.).color(MUTED));
                    let response=ui.label(RichText::new(&value).size(if value.chars().count()>12 {19.}else{22.}).strong().color(if label=="Średnia ocen" {accent.color()}else{TEXT}));
                    if label=="Średnia ocen" {response.on_hover_text("Średnia arytmetyczna / ważona. Oceny cząstkowe, bez zachowania. Nieustawiona waga = 1.");}
                });
            }
        });
    });
    ui.add_space(6.);
}
pub fn behaviour(ui: &mut egui::Ui, payload: Option<&Payload>, accent: Accent) {
    if let Some(Payload::Grades {
        behaviour,
        behaviour_error,
        ..
    }) = payload
    {
        if let Some(error) = behaviour_error {
            ui.colored_label(egui::Color32::from_rgb(255, 194, 99), "!")
                .on_hover_text(error);
        }
        for grade in behaviour.iter().rev() {
            egui::Frame::none()
                .fill(accent.color().gamma_multiply(0.15))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    accent.color().gamma_multiply(0.4),
                ))
                .rounding(4.)
                .inner_margin(egui::Margin::symmetric(9., 6.))
                .show(ui, |ui| {
                    ui.label(RichText::new(&grade.value).strong().color(accent.color()));
                })
                .response
                .on_hover_text(format!(
                    "{}\nSemestr {} · {}",
                    grade.description, grade.semester, grade.date
                ));
        }
        if behaviour.is_empty() {
            ui.label("—");
        }
    } else {
        ui.label("—");
    }
    ui.label(RichText::new("Zachowanie").color(MUTED));
}
fn matches(search: &str, parts: &[&str]) -> bool {
    let search = search.to_lowercase();
    search.is_empty() || parts.iter().any(|p| p.to_lowercase().contains(&search))
}
fn short_date(date: &str) -> String {
    if let Some(parsed) = date
        .get(..10)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    {
        let day = parsed.format("%d.%m.%Y").to_string();
        if let Some(time) = date.get(11..16).filter(|s| *s != "00:00") {
            return format!("{day}  {time}");
        }
        return day;
    }
    date.chars().take(16).collect::<String>().replace('T', " ")
}
pub(crate) fn grade_color(value: &str, accent: Accent) -> egui::Color32 {
    match value.chars().next().and_then(|c| c.to_digit(10)) {
        Some(5..=6) => GREEN,
        Some(1..=2) => RED,
        _ => accent.color(),
    }
}

pub struct Reader<'a> {
    pub selected: Option<&'a str>,
    pub detail: Option<&'a Message>,
    pub error: Option<&'a str>,
    pub loading: bool,
}

pub fn render(
    ui: &mut egui::Ui,
    payload: &Payload,
    accent: Accent,
    state: &mut ViewState,
    reader: Reader<'_>,
    week: NaiveDate,
) -> Option<String> {
    match payload {
        Payload::Grades { items, .. } => grades(ui, items, accent, state),
        Payload::Attendance(items) => attendance(ui, items, accent, state),
        Payload::Notices(items) => notices(ui, items, accent, state),
        Payload::Timetable(items) => timetable(ui, items, accent, week),
        Payload::Messages { items, .. } => {
            return messages(
                ui,
                items,
                accent,
                &state.search,
                reader.selected,
                reader.detail,
                reader.error,
                reader.loading,
            )
        }
    }
    None
}

pub(crate) fn average_pair<'a>(
    items: impl Iterator<Item = &'a Grade> + Clone,
    semester: usize,
) -> String {
    let display = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.2}").replace('.', ","))
            .unwrap_or_else(|| "—".into())
    };
    format!(
        "[{}]/[{}]",
        display(data::grade_average(items.clone(), semester, false)),
        display(data::grade_average(items, semester, true))
    )
}
fn grades(ui: &mut egui::Ui, items: &[Grade], accent: Accent, state: &mut ViewState) {
    let subjects: BTreeSet<_> = items.iter().map(|g| &g.subject).collect();
    stats(
        ui,
        accent,
        [
            ("Wpisane oceny", items.len().to_string()),
            ("Przedmioty", subjects.len().to_string()),
            ("Średnia ocen", average_pair(items.iter(), state.semester)),
        ],
    );
    ui.horizontal(|ui| {
        dim(ui, "TWOJE PRZEDMIOTY");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt("semester")
                .selected_text(match state.semester {
                    1 => "Semestr 1",
                    2 => "Semestr 2",
                    _ => "Cały rok",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.semester, 0, "Cały rok");
                    ui.selectable_value(&mut state.semester, 1, "Semestr 1");
                    ui.selectable_value(&mut state.semester, 2, "Semestr 2");
                });
            ui.small(RichText::new("Średnie: arytm. / ważona").color(MUTED));
        });
    });
    let mut grouped: BTreeMap<&str, Vec<&Grade>> = BTreeMap::new();
    for g in items {
        if (state.semester == 0 || g.semester == state.semester.to_string())
            && matches(&state.search, &[&g.subject, &g.category, &g.teacher])
        {
            grouped.entry(&g.subject).or_default().push(g);
        }
    }
    if grouped.is_empty() {
        empty(
            ui,
            accent,
            "Brak ocen do wyświetlenia",
            "Nie ma jeszcze wpisów albo żaden nie pasuje do filtrów.",
        );
    }
    if !grouped.is_empty() {
        crate::compact::grades(ui, &grouped, accent, state);
    }
}

fn attendance_type(a: &Attendance) -> usize {
    let kind = a
        .kind
        .to_lowercase()
        .replace('ó', "o")
        .replace(['ź', 'ż'], "z");
    if kind.contains("spozn") {
        4
    } else if kind == "z" || kind.contains("zwoln") {
        3
    } else if kind.contains("nieuspraw") {
        1
    } else if kind == "nu" || kind.contains("uspraw") {
        2
    } else if kind == "n" || kind.contains("nieobecn") {
        1
    } else if kind == "o" || kind.contains("obecn") || a.present == Some(true) {
        0
    } else {
        4
    }
}
fn attendance_days<'a>(
    items: &'a [Attendance],
    state: &ViewState,
) -> BTreeMap<&'a str, Vec<&'a Attendance>> {
    let mut days: BTreeMap<&str, Vec<&Attendance>> = BTreeMap::new();
    for entry in items {
        days.entry(entry.date.get(..10).unwrap_or(&entry.date))
            .or_default()
            .push(entry);
    }
    days.retain(|date, entries| {
        entries
            .iter()
            .any(|a| state.attendance_types[attendance_type(a)])
            && (matches(&state.search, &[date])
                || entries
                    .iter()
                    .any(|a| matches(&state.search, &[&a.subject, &a.kind])))
    });
    for entries in days.values_mut() {
        entries.sort_by_key(|a| a.lesson.parse::<u32>().unwrap_or(u32::MAX));
    }
    days
}

fn attendance(ui: &mut egui::Ui, items: &[Attendance], accent: Accent, state: &mut ViewState) {
    let present = items.iter().filter(|a| a.present == Some(true)).count();
    let known = items.iter().filter(|a| a.present.is_some()).count();
    let percentage = if known > 0 {
        format!("{:.1}%", present as f64 * 100. / known as f64)
    } else {
        "—".into()
    };
    stats(
        ui,
        accent,
        [
            ("Obecność we wpisach", percentage),
            ("Obecności", present.to_string()),
            ("Pozostałe wpisy", (items.len() - present).to_string()),
        ],
    );
    const LABELS: [&str; 5] = [
        "O · Obecny",
        "N · Nieobecny",
        "NU · Nieobecny usprawiedliwiony",
        "Z · Zwolniony",
        "Inne / spóźnienia",
    ];
    const CODES: [&str; 5] = ["O", "N", "NU", "Z", "?"];
    let colors = [
        GREEN,
        RED,
        egui::Color32::from_rgb(125, 185, 255),
        egui::Color32::from_rgb(255, 194, 99),
        MUTED,
    ];
    ui.horizontal_wrapped(|ui| {
        ui.menu_button(
            format!(
                "Typy frekwencji · {}/5 ▾",
                state.attendance_types.iter().filter(|v| **v).count()
            ),
            |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("Wszystkie").clicked() {
                        state.attendance_types = [true; 5];
                    }
                    if ui.small_button("Żadne").clicked() {
                        state.attendance_types = [false; 5];
                    }
                });
                ui.separator();
                for (i, label) in LABELS.iter().enumerate() {
                    ui.checkbox(&mut state.attendance_types[i], *label);
                }
            },
        );
        ui.small(
            RichText::new("Dni zawierające dowolny zaznaczony typ · pokazujemy cały dzień")
                .color(MUTED),
        );
    });
    ui.horizontal_wrapped(|ui| {
        for i in 0..4 {
            ui.colored_label(colors[i], LABELS[i]);
        }
    });
    let days = attendance_days(items, state);
    if days.is_empty() {
        empty(
            ui,
            accent,
            "Brak pasujących dni",
            "Zaznacz typy frekwencji lub zmień wyszukiwanie.",
        );
    }
    for (date, entries) in days.iter().rev() {
        theme::glass(accent).inner_margin(12.).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.allocate_ui(Vec2::new(115., 42.), |ui| {
                    ui.label(RichText::new(short_date(date)).strong());
                    if let Ok(day) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                        ui.small(
                            [
                                "Poniedziałek",
                                "Wtorek",
                                "Środa",
                                "Czwartek",
                                "Piątek",
                                "Sobota",
                                "Niedziela",
                            ][day.weekday().num_days_from_monday() as usize],
                        );
                    }
                });
                for a in entries {
                    let kind = attendance_type(a);
                    ui.vertical(|ui| {
                        ui.small(RichText::new(format!("Lekcja {}", a.lesson)).color(MUTED));
                        let code = if kind == 4 && a.kind.to_lowercase().contains("spóź") {
                            "S"
                        } else {
                            CODES[kind]
                        };
                        theme::badge(ui, code, colors[kind]);
                    })
                    .response
                    .on_hover_text(format!(
                        "{} · lekcja {}\n{}\n{}",
                        a.date, a.lesson, a.subject, a.kind
                    ));
                }
            });
        });
        ui.add_space(3.);
    }
}

fn notices(ui: &mut egui::Ui, items: &[Notice], accent: Accent, state: &ViewState) {
    let unread = items.iter().filter(|n| n.unread).count();
    stats(
        ui,
        accent,
        [
            ("Ogłoszenia", items.len().to_string()),
            ("Nieprzeczytane", unread.to_string()),
            ("Źródło", "Twoja szkoła".into()),
        ],
    );
    let filtered: Vec<_> = items
        .iter()
        .filter(|n| matches(&state.search, &[&n.title, &n.content, &n.author]))
        .collect();
    if filtered.is_empty() {
        empty(
            ui,
            accent,
            "Spokojnie, nic nowego",
            "Brak ogłoszeń pasujących do wyszukiwania.",
        );
    }
    for (index, n) in filtered.into_iter().enumerate() {
        theme::glass(accent).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if n.unread {
                    theme::badge(ui, "NOWE", accent.color());
                }
                dim(ui, format!("{}  ·  {}", short_date(&n.date), n.author));
            });
            egui::CollapsingHeader::new(RichText::new(&n.title).size(19.).strong())
                .id_salt(&n.id)
                .default_open(index == 0)
                .show(ui, |ui| {
                    ui.add_space(8.);
                    ui.label(&n.content);
                    if !n.until.is_empty() {
                        ui.add_space(8.);
                        ui.small(
                            RichText::new(format!("Ważne do: {}", short_date(&n.until)))
                                .color(MUTED),
                        );
                    }
                });
        });
        ui.add_space(4.);
    }
}

fn timetable(ui: &mut egui::Ui, items: &[Lesson], accent: Accent, week: NaiveDate) {
    let active = items.iter().filter(|l| !l.canceled).count();
    stats(
        ui,
        accent,
        [
            ("Lekcje w tygodniu", active.to_string()),
            (
                "Zastępstwa",
                items.iter().filter(|l| l.substitution).count().to_string(),
            ),
            (
                "Odwołane",
                items.iter().filter(|l| l.canceled).count().to_string(),
            ),
        ],
    );
    crate::compact::timetable(ui, items, accent, week);
}

#[allow(clippy::too_many_arguments)]
fn messages(
    ui: &mut egui::Ui,
    items: &[Message],
    accent: Accent,
    search: &str,
    selected: Option<&str>,
    detail: Option<&Message>,
    detail_error: Option<&str>,
    loading: bool,
) -> Option<String> {
    let mut clicked = None;
    let filtered: Vec<_> = items
        .iter()
        .filter(|m| matches(search, &[&m.title, &m.sender, &m.content]))
        .collect();
    let total_width = ui.available_width();
    let gap = 16.;
    let list_width = (total_width * 0.35)
        .clamp(210., 460.)
        .min(total_width * 0.46);
    let pane_height = (ui.clip_rect().bottom() - ui.cursor().top() - 8.).max(100.);
    let (area, _) =
        ui.allocate_exact_size(Vec2::new(total_width, pane_height), egui::Sense::hover());
    let list_rect = egui::Rect::from_min_size(area.min, Vec2::new(list_width, pane_height));
    let reader_rect =
        egui::Rect::from_min_max(egui::pos2(list_rect.right() + gap, area.top()), area.max);
    let color = accent.color();
    let list_fill = theme::blend(egui::Color32::from_rgb(18, 23, 33), color, 0.17);
    let reader_fill = theme::blend(egui::Color32::from_rgb(18, 23, 33), color, 0.035);
    ui.painter().rect(
        list_rect,
        8.,
        list_fill,
        egui::Stroke::new(1_f32, theme::blend(list_fill, color, 0.45)),
    );
    ui.painter().rect(
        reader_rect,
        8.,
        reader_fill,
        egui::Stroke::new(1_f32, theme::blend(reader_fill, color, 0.22)),
    );
    ui.painter().rect_filled(
        egui::Rect::from_min_size(reader_rect.min + Vec2::new(0., 12.), Vec2::new(3., 28.)),
        1.,
        color,
    );
    let mut list_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("message_list_panel")
            .max_rect(list_rect.shrink(12.))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    list_ui.set_clip_rect(list_rect.shrink(1.).intersect(ui.clip_rect()));
    list_ui.horizontal(|ui| {
        ui.label(RichText::new("ODEBRANE").size(12.).strong().color(color));
        ui.small(format!("{} / {}", filtered.len(), items.len()));
    });
    list_ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("inbox_list")
        .auto_shrink([false, false])
        .max_height(list_ui.available_height())
        .show(&mut list_ui, |ui| {
            if filtered.is_empty() {
                ui.label("Brak pasujących wiadomości.");
            }
            for m in filtered {
                let chosen = selected == Some(m.id.as_str());
                let card_fill = theme::blend(
                    egui::Color32::from_rgb(22, 28, 39),
                    color,
                    if chosen { 0.36 } else { 0.055 },
                );
                let card = egui::Frame::none()
                    .rounding(5.)
                    .inner_margin(11.)
                    .fill(card_fill)
                    .stroke(egui::Stroke::new(
                        if chosen { 2_f32 } else { 1_f32 },
                        if chosen {
                            color
                        } else {
                            theme::blend(card_fill, color, 0.2)
                        },
                    ))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.small(RichText::new(short_date(&m.date)).color(if chosen {
                                TEXT
                            } else {
                                MUTED
                            }));
                            if m.unread {
                                ui.colored_label(color, "● Nowa");
                            }
                        });
                        ui.add(
                            egui::Label::new(RichText::new(&m.title).size(16.).strong()).truncate(),
                        )
                        .on_hover_text(&m.title);
                        ui.add(
                            egui::Label::new(RichText::new(&m.sender).size(12.).color(if chosen {
                                TEXT
                            } else {
                                MUTED
                            }))
                            .truncate(),
                        )
                        .on_hover_text(&m.sender);
                        let snippet = m.content.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !snippet.is_empty() {
                            ui.add(
                                egui::Label::new(RichText::new(snippet).size(12.).color(MUTED))
                                    .truncate(),
                            );
                        }
                        if m.has_files {
                            ui.small(RichText::new("Załączniki").color(color));
                        }
                    });
                if chosen {
                    let stripe = egui::Rect::from_min_max(
                        card.response.rect.min + Vec2::new(0., 7.),
                        egui::pos2(
                            card.response.rect.left() + 4.,
                            card.response.rect.bottom() - 7.,
                        ),
                    );
                    ui.painter().rect_filled(stripe, 2., color);
                }
                let response = card.response.interact(egui::Sense::click());
                if response
                    .clone()
                    .on_hover_text("Otwórz wiadomość. Librus może oznaczyć ją jako przeczytaną.")
                    .clicked()
                {
                    clicked = Some(m.id.clone());
                }
                if response.hovered() {
                    ui.painter()
                        .rect_stroke(response.rect, 5., egui::Stroke::new(1_f32, color));
                }
                ui.add_space(2.);
            }
        });
    let mut reader_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("message_reader_panel")
            .max_rect(reader_rect.shrink(18.))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    reader_ui.set_clip_rect(reader_rect.shrink(1.).intersect(ui.clip_rect()));
    reader_ui.label(
        RichText::new("PODGLĄD WIADOMOŚCI")
            .size(12.)
            .strong()
            .color(color),
    );
    reader_ui.separator();
    egui::ScrollArea::vertical()
        .id_salt(("message_content", selected))
        .auto_shrink([false, false])
        .max_height(reader_ui.available_height())
        .show(&mut reader_ui, |ui| {
            if loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Otwieranie wiadomości…");
                });
            }
            if let Some(error) = detail_error {
                ui.colored_label(RED, error);
            }
            if let Some(message) = detail {
                ui.label(RichText::new(&message.title).size(23.).strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&message.sender).color(color));
                    ui.small(RichText::new(short_date(&message.date)).color(MUTED));
                });
                ui.add_space(8.);
                ui.separator();
                ui.add_space(8.);
                ui.add(egui::Label::new(RichText::new(&message.content).size(16.)).wrap());
                if !message.attachments.is_empty() {
                    ui.add_space(18.);
                    ui.separator();
                    ui.label(RichText::new("ZAŁĄCZNIKI").size(12.).strong().color(color));
                    for name in &message.attachments {
                        ui.label(name);
                    }
                    ui.hyperlink_to(
                        "Otwórz załączniki w Librusie",
                        "https://wiadomosci.librus.pl/nowy/inbox",
                    );
                }
            } else if !loading && detail_error.is_none() {
                ui.add_space(22.);
                ui.label(
                    RichText::new("Wybierz wiadomość z listy")
                        .size(22.)
                        .strong(),
                );
                dim(ui, "Tutaj przeczytasz jej treść i zobaczysz załączniki.");
            }
        });
    clicked
}

pub fn unloaded(ui: &mut egui::Ui, section: Section, accent: Accent, loading: bool) {
    empty(
        ui,
        accent,
        if loading {
            "Pobieranie danych…"
        } else {
            "Gotowe, kiedy Ty"
        },
        if loading {
            "Łączę się z Twoim dziennikiem. To może potrwać chwilę."
        } else {
            section.subtitle()
        },
    );
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn message_panes_use_full_viewport_and_distinct_accent_surfaces() {
        for accent in Accent::ALL {
            for size in [Vec2::new(600., 430.), Vec2::new(1450., 1100.)] {
                let Payload::Messages { mut items, .. } =
                    crate::demo::payloads(data::monday(data::today()))
                        .remove(&Section::Messages)
                        .unwrap()
                else {
                    panic!("Missing messages fixture");
                };
                let mut detail = items[0].clone();
                detail.content =
                    "Długa treść wiadomości, która musi przewijać się w podglądzie.\n".repeat(150);
                for i in 0..30 {
                    let mut entry = detail.clone();
                    entry.id = format!("fixture-{i}");
                    items.push(entry);
                }
                let ctx = egui::Context::default();
                theme::apply(&ctx, accent);
                let mut bounds = egui::Rect::NOTHING;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            bounds = ui.available_rect_before_wrap();
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    messages(
                                        ui,
                                        &items,
                                        accent,
                                        "",
                                        Some(&detail.id),
                                        Some(&detail),
                                        None,
                                        false,
                                    );
                                    assert!(
                                        ui.min_rect().bottom() <= bounds.bottom() + 1.,
                                        "The outer page must not scroll"
                                    );
                                    assert!(ui.min_rect().right() <= bounds.right() + 1.);
                                });
                        });
                    },
                );
                let base = egui::Color32::from_rgb(18, 23, 33);
                let fills = [
                    theme::blend(base, accent.color(), 0.17),
                    theme::blend(base, accent.color(), 0.035),
                ];
                let panes: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        if let egui::Shape::Rect(rect) = &shape.shape {
                            if fills.contains(&rect.fill) {
                                return Some((rect.rect, rect.fill));
                            }
                        }
                        None
                    })
                    .collect();
                assert_eq!(panes.len(), 2);
                assert_ne!(panes[0].1, panes[1].1);
                for (rect, _) in &panes {
                    assert!(
                        rect.height() >= bounds.height() - 24.,
                        "Pane must fill the viewport: pane {rect:?}, viewport {bounds:?}"
                    );
                    assert!(rect.bottom() <= bounds.bottom() + 1.);
                }
                assert!((panes[1].0.right() - bounds.right()).abs() < 1.);
                assert!((panes[1].0.left() - panes[0].0.right() - 16.).abs() < 1.);
            }
        }
    }
    #[test]
    fn attendance_filters_match_days_with_any_selected_type_and_keep_context() {
        let entry = |date: &str, lesson: &str, kind: &str| Attendance {
            date: date.into(),
            lesson: lesson.into(),
            subject: "Matematyka".into(),
            kind: kind.into(),
            present: None,
        };
        let items = vec![
            entry("2026-11-09", "2", "Obecność"),
            entry("2026-11-09", "1", "Nieobecność nieusprawiedliwiona"),
            entry("2026-11-10", "1", "Nieobecność usprawiedliwiona"),
            entry("2026-11-11", "1", "Zwolnienie"),
            entry("2026-11-12", "1", "Spóźnienie usprawiedliwione"),
        ];
        assert_eq!(
            items.iter().map(attendance_type).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
        let mut state = ViewState {
            attendance_types: [false, true, false, true, false],
            ..Default::default()
        };
        let days = attendance_days(&items, &state);
        assert_eq!(
            days.keys().copied().collect::<Vec<_>>(),
            vec!["2026-11-09", "2026-11-11"]
        );
        assert_eq!(days["2026-11-09"].len(), 2);
        assert_eq!(days["2026-11-09"][0].lesson, "1");
        state.attendance_types = [false; 5];
        assert!(attendance_days(&items, &state).is_empty());
        state.attendance_types = [true; 5];
        assert_eq!(attendance_days(&items, &state).len(), 4);
    }
    #[test]
    fn grades_and_week_fit_small_and_large_viewports() {
        for size in [Vec2::new(600., 430.), Vec2::new(1050., 650.)] {
            for section in [Section::Grades, Section::Timetable] {
                let week = data::monday(data::today());
                let mut payload = crate::demo::payloads(week).remove(&section).unwrap();
                if let Payload::Grades { items, .. } = &mut payload {
                    let mut many = Vec::new();
                    for subject in 0..40 {
                        for grade in items.iter().take(4) {
                            let mut g = grade.clone();
                            g.subject = format!("Przedmiot {subject:02}");
                            many.push(g);
                        }
                    }
                    *items = many;
                }
                let ctx = egui::Context::default();
                theme::apply(&ctx, Accent::Cobalt);
                let _=ctx.run(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,size)),..Default::default()},|ctx| {
                    egui::CentralPanel::default().show(ctx,|ui| {
                        let bottom=ui.clip_rect().bottom();let right=ui.clip_rect().right();
                        // Match the scroll container used by the application.
                        egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui,|ui| {
                            match &payload {
                                Payload::Grades{items,..} => grades(ui,items,Accent::Cobalt,&mut ViewState::default()),
                                Payload::Timetable(items) => timetable(ui,items,Accent::Cobalt,week),
                                _=>unreachable!(),
                            }
                            assert!(ui.min_rect().bottom() <= bottom+1.,"{section:?}: content must fit vertically: {:?}, bottom {bottom}",ui.min_rect());
                            assert!(ui.min_rect().right() <= right+1.,"Content must fit horizontally");
                        });
                    });
                });
            }
        }
    }
    #[test]
    fn compact_heading_fits_one_row_in_every_tab() {
        for width in [600., 1100.] {
            for section in [
                Section::Timetable,
                Section::Grades,
                Section::Attendance,
                Section::Messages,
                Section::Notices,
                Section::Account,
            ] {
                let ctx = egui::Context::default();
                theme::apply(&ctx, Accent::Cobalt);
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width + 16., 800.),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let response = heading(
                                ui,
                                section,
                                Accent::Cobalt,
                                Some("10.11.2026 17:14"),
                                false,
                            );
                            assert!(
                                response.rect.height() <= 36.,
                                "Header should remain a single compact row"
                            );
                            assert!(response.rect.right() <= ui.max_rect().right() + 1.);
                        });
                    },
                );
            }
        }
    }
    #[test]
    fn day_columns_are_separate_and_days_without_classes_are_fully_red() {
        let week = data::monday(data::today());
        let Payload::Timetable(mut lessons) = crate::demo::payloads(week)
            .remove(&Section::Timetable)
            .unwrap()
        else {
            panic!("Missing fixture")
        };
        let thursday = (week + Duration::days(3)).to_string();
        let friday = (week + Duration::days(4)).to_string();
        lessons.retain(|lesson| lesson.date != friday);
        for lesson in &mut lessons {
            if lesson.date == thursday {
                lesson.canceled = true;
            }
        }
        let ctx = egui::Context::default();
        theme::apply(&ctx, Accent::Cobalt);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1400., 900.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| timetable(ui, &lessons, Accent::Cobalt, week));
            },
        );
        fn panels(shape: &egui::Shape, out: &mut Vec<(egui::Rect, egui::Color32)>) {
            match shape {
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        panels(shape, out);
                    }
                }
                egui::Shape::Rect(rect)
                    if rect.fill.a() == 255
                        && rect.rect.height() > 250.
                        && (150. ..350.).contains(&rect.rect.width()) =>
                {
                    out.push((rect.rect, rect.fill))
                }
                _ => {}
            }
        }
        let mut days = vec![];
        fn has_fill(shape: &egui::Shape, color: egui::Color32) -> bool {
            match shape {
                egui::Shape::Rect(rect) => rect.fill == color,
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| has_fill(s, color)),
                _ => false,
            }
        }
        assert!(
            output
                .shapes
                .iter()
                .any(|s| has_fill(&s.shape, egui::Color32::from_rgb(133, 70, 25))),
            "Substitutions need a full orange tile"
        );
        assert!(
            output
                .shapes
                .iter()
                .any(|s| has_fill(&s.shape, egui::Color32::from_rgb(115, 36, 48))),
            "Cancellations need a full red tile"
        );
        for shape in &output.shapes {
            panels(&shape.shape, &mut days);
        }
        days.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()));
        assert_eq!(days.len(), 5);
        for pair in days.windows(2) {
            assert!(pair[1].0.left() - pair[0].0.right() >= 17.);
            assert!((pair[1].0.height() - pair[0].0.height()).abs() < 1.);
        }
        for (_, fill) in &days[3..] {
            assert!(
                fill.r() > fill.g() * 2 && fill.r() > fill.b(),
                "A day with no active lessons must have a full red panel"
            );
        }
    }
}
