use crate::i18n::{tr, trf};
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Section {
    Grades,
    Attendance,
    Messages,
    Notices,
    Timetable,
    Account,
}
impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Self::Grades => tr!("Oceny", "Grades"),
            Self::Attendance => tr!("Frekwencja", "Attendance"),
            Self::Messages => tr!("Wiadomości", "Messages"),
            Self::Notices => tr!("Ogłoszenia", "Notices"),
            Self::Timetable => tr!("Plan lekcji", "Timetable"),
            Self::Account => tr!("Twoje konto", "Profile"),
        }
    }
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Grades => tr!(
                "Oceny z przedmiotów i podsumowanie średnich",
                "Subject grades and average summaries"
            ),
            Self::Attendance => tr!(
                "Obecności, nieobecności i historia wpisów",
                "Attendance, absences and entry history"
            ),
            Self::Messages => tr!(
                "Skrzynka odbiorcza i treść wiadomości",
                "Inbox and message content"
            ),
            Self::Notices => tr!(
                "Informacje i komunikaty od szkoły",
                "Updates and announcements from your school"
            ),
            Self::Timetable => tr!(
                "Twój tydzień i zmiany w planie",
                "Your week and timetable changes"
            ),
            Self::Account => tr!(
                "Logowanie i ustawienia wyglądu",
                "Sign-in, language and appearance settings"
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grade {
    #[serde(default)]
    pub id: String,
    pub subject: String,
    pub value: String,
    pub category: String,
    pub date: String,
    pub teacher: String,
    pub semester: String,
    pub numeric: Option<f64>,
    pub counts: bool,
    pub weight: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BehaviourGrade {
    pub value: String,
    pub description: String,
    pub semester: String,
    pub date: String,
}

pub fn grade_number(raw: &str, plus: f64, minus: f64, count_zero: bool) -> Option<f64> {
    let value = raw.trim().replace('−', "-");
    let (base, modifier) = if let Some(base) = value.strip_suffix('+') {
        (base, plus)
    } else if let Some(base) = value.strip_suffix('-') {
        (base, -minus)
    } else {
        (value.as_str(), 0.)
    };
    let base = base.replace(',', ".").parse::<f64>().ok()?;
    if base == 0. {
        return if count_zero && modifier == 0. {
            Some(0.)
        } else {
            None
        };
    }
    let number = base + modifier;
    if (1. ..=6.).contains(&base) && number > 0. && number <= 6.5 && number.is_finite() {
        Some(number)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradeBand {
    Neutral,
    Low,
    Middle,
    High,
}

/// Visual ranges follow the written grade, independently of school-specific
/// numeric settings used when calculating averages. In particular 3- is the
/// first amber grade, while 5- stays amber until a full 5 is reached.
pub fn grade_band(value: &str) -> GradeBand {
    match grade_number(value, 0.5, 0.25, true) {
        Some(number) if number >= 5. => GradeBand::High,
        Some(number) if number >= 2.75 => GradeBand::Middle,
        Some(number) if number > 0. => GradeBand::Low,
        _ => GradeBand::Neutral,
    }
}

pub fn grade_average<'a>(
    items: impl IntoIterator<Item = &'a Grade>,
    semester: usize,
    weighted: bool,
) -> Option<f64> {
    let mut sum = 0.;
    let mut weights = 0.;
    for item in items
        .into_iter()
        .filter(|g| g.counts && (semester == 0 || g.semester == semester.to_string()))
    {
        if let Some(number) = item.numeric.filter(|n| n.is_finite()) {
            let weight = if weighted { item.weight? } else { 1. };
            if weight.is_finite() && weight > 0. {
                sum += number * weight;
                weights += weight;
            }
        }
    }
    if weights > 0. {
        Some(sum / weights)
    } else {
        None
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attendance {
    pub date: String,
    pub lesson: String,
    pub subject: String,
    pub kind: String,
    pub present: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notice {
    pub id: String,
    pub title: String,
    pub content: String,
    pub date: String,
    pub until: String,
    pub author: String,
    pub unread: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lesson {
    pub date: String,
    pub number: String,
    pub subject: String,
    pub teacher: String,
    pub room: String,
    pub from: String,
    pub to: String,
    pub canceled: bool,
    pub substitution: bool,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original: Option<OriginalLesson>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalLesson {
    pub date: String,
    pub number: String,
    pub subject: String,
    pub teacher: String,
    pub room: String,
    pub from: String,
    pub to: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub title: String,
    pub sender: String,
    pub content: String,
    pub date: String,
    pub unread: bool,
    pub attachments: Vec<String>,
    pub has_files: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub enum Payload {
    Grades {
        items: Vec<Grade>,
        behaviour: Vec<BehaviourGrade>,
        behaviour_error: Option<String>,
    },
    Attendance(Vec<Attendance>),
    Notices(Vec<Notice>),
    Timetable(Vec<Lesson>),
    Messages {
        items: Vec<Message>,
        total: usize,
    },
}
impl Payload {
    pub fn len(&self) -> usize {
        match self {
            Self::Grades { items, .. } => items.len(),
            Self::Attendance(x) => x.len(),
            Self::Notices(x) => x.len(),
            Self::Timetable(x) => x.len(),
            Self::Messages { items, .. } => items.len(),
        }
    }
}

pub fn today() -> NaiveDate {
    Utc::now()
        .with_timezone(&chrono_tz::Europe::Warsaw)
        .date_naive()
}
pub fn monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}
pub fn scalar(value: &Value) -> String {
    match value {
        Value::String(x) => x.clone(),
        Value::Number(x) => x.to_string(),
        _ => String::new(),
    }
}
pub fn text(value: &Value, key: &str) -> String {
    scalar(&value[key])
}
pub fn flag(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| value.as_u64() == Some(1) || value.as_str() == Some("1"))
}
pub fn records<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| {
        anyhow::anyhow!(trf!(
            "Librus zwrócił nieoczekiwany format danych ({key}).",
            "Librus returned an unexpected data format ({key})."
        ))
    })
}
pub fn names(values: &[Value]) -> HashMap<String, String> {
    values
        .iter()
        .map(|v| {
            let name = if !text(v, "Name").is_empty() {
                text(v, "Name")
            } else {
                format!("{} {}", text(v, "FirstName"), text(v, "LastName"))
                    .trim()
                    .to_owned()
            };
            (text(v, "Id"), name)
        })
        .collect()
}
pub fn resolve(value: &Value, names: &HashMap<String, String>, fallback: &str) -> String {
    if let Some(name) = value
        .get("Name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        return name.to_owned();
    }
    let person = format!("{} {}", text(value, "FirstName"), text(value, "LastName"))
        .trim()
        .to_owned();
    if !person.is_empty() {
        return person;
    }
    let id = text(value, "Id");
    names
        .get(&id)
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| {
            if id.is_empty() {
                fallback.to_owned()
            } else {
                format!("{fallback} #{id}")
            }
        })
}

pub fn plain_html(html: &str) -> String {
    let html = if let Some((_, content)) = html.split_once("<![CDATA[") {
        content.split_once("]]>").map(|(s, _)| s).unwrap_or(content)
    } else {
        html
    };
    let mut result = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        result.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>').map(|n| start + n) else {
            result.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let tag = rest[start + 1..end].trim().to_ascii_lowercase();
        if tag.starts_with("script") || tag.starts_with("style") {
            let close = if tag.starts_with("script") {
                "</script>"
            } else {
                "</style>"
            };
            let tail = &rest[end + 1..];
            if let Some(stop) = tail.to_ascii_lowercase().find(close) {
                rest = &tail[stop + close.len()..];
                continue;
            }
            rest = "";
            break;
        }
        if tag.starts_with("br")
            || ["/p", "/div", "/li", "/tr", "/h1", "/h2", "/h3"].contains(&tag.as_str())
        {
            result.push('\n');
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    let decoded = html_escape::decode_html_entities(&result);
    let mut out = String::new();
    let mut empty = false;
    for line in decoded.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !empty && !out.is_empty() {
                out.push('\n');
            }
            empty = true;
        } else {
            out.push_str(line);
            out.push('\n');
            empty = false;
        }
    }
    out.trim().to_owned()
}
pub fn decode_message(raw: &str) -> String {
    let decoded = STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    plain_html(decoded.as_deref().unwrap_or(raw))
}
pub fn message(value: &Value, full: bool) -> Message {
    let raw = if full {
        text(value, "Message")
    } else {
        text(value, "content")
    };
    Message {
        id: text(value, "messageId"),
        title: text(value, "topic"),
        sender: text(value, "senderName"),
        content: decode_message(&raw),
        date: text(value, "sendDate"),
        unread: value["readDate"].is_null() || text(value, "readDate").is_empty(),
        attachments: value["attachments"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|a| {
                        ["name", "filename", "fileName", "originalName"]
                            .into_iter()
                            .map(|key| text(a, key))
                            .find(|s| !s.is_empty())
                            .unwrap_or_else(|| tr!("Załącznik", "Attachment").into())
                    })
                    .collect()
            })
            .unwrap_or_default(),
        has_files: flag(&value["isAnyFileAttached"]),
    }
}

fn room_label(value: &Value) -> Option<String> {
    // A room can be a named workshop or venue, not just its short symbol.
    ["Name", "Symbol", "Number"]
        .into_iter()
        .map(|field| text(value, field).trim().to_owned())
        .find(|label| !label.is_empty())
}

pub fn classroom_names(records: &[Value]) -> HashMap<String, String> {
    records
        .iter()
        .filter_map(|room| {
            let id = text(room, "Id").trim().to_owned();
            room_label(room)
                .filter(|_| !id.is_empty())
                .map(|label| (id, label))
        })
        .collect()
}

fn classroom(lesson: &Value, rooms: &HashMap<String, String>) -> String {
    // Prefer full names, inline or resolved by ID, before shortened symbols.
    // An unavailable lookup must not hide a usable label in another field.
    let candidates: Vec<_> = [lesson, &lesson["Lesson"]]
        .into_iter()
        .flat_map(|source| ["Classroom", "ClassRoom", "Room"].map(|key| &source[key]))
        .collect();
    resolve_classroom(&candidates, rooms)
}

fn resolve_classroom(candidates: &[&Value], rooms: &HashMap<String, String>) -> String {
    for value in candidates {
        let name = text(value, "Name").trim().to_owned();
        if !name.is_empty() {
            return name;
        }
    }
    for value in candidates {
        if !value.is_object() {
            let raw = scalar(value).trim().to_owned();
            if !raw.is_empty() {
                return rooms
                    .get(&raw)
                    .filter(|name| !name.trim().is_empty())
                    .map(|name| name.trim().to_owned())
                    .unwrap_or(raw);
            }
        }
    }
    for value in candidates {
        let id = text(value, "Id").trim().to_owned();
        if let Some(name) = rooms.get(&id).filter(|name| !name.trim().is_empty()) {
            return name.trim().to_owned();
        }
    }
    for value in candidates {
        if let Some(label) = room_label(value) {
            return label;
        }
    }
    let mut fallback = None;
    for value in candidates {
        let id = text(value, "Id").trim().to_owned();
        if id.is_empty() {
            continue;
        }
        fallback.get_or_insert(id);
    }
    fallback.unwrap_or_else(|| "—".to_owned())
}

fn original_lesson(
    value: &Value,
    lesson: &Lesson,
    subjects: &HashMap<String, String>,
    teachers: &HashMap<String, String>,
    rooms: &HashMap<String, String>,
) -> Option<OriginalLesson> {
    let subject = resolve(&value["OrgSubject"], subjects, "");
    let teacher = resolve(&value["OrgTeacher"], teachers, "");
    let room = resolve_classroom(&[&value["OrgClassroom"]], rooms);
    let field = |key, fallback: &str| {
        let text = text(value, key);
        if text.is_empty() {
            fallback.to_owned()
        } else {
            text
        }
    };
    let original = OriginalLesson {
        date: field("OrgDate", &lesson.date),
        number: field("OrgLessonNo", &lesson.number),
        subject,
        teacher,
        room,
        from: field("OrgHourFrom", &lesson.from),
        to: field("OrgHourTo", &lesson.to),
    };
    (!original.subject.is_empty()
        || !original.teacher.is_empty()
        || original.room != "—"
        || original.date != lesson.date
        || original.number != lesson.number
        || original.from != lesson.from
        || original.to != lesson.to)
        .then_some(original)
}

pub fn timetable(
    value: &Value,
    subjects: &HashMap<String, String>,
    teachers: &HashMap<String, String>,
    rooms: &HashMap<String, String>,
) -> Result<Vec<Lesson>> {
    let root = value.get("Timetable").ok_or_else(|| {
        anyhow::anyhow!(tr!(
            "Brak planu w odpowiedzi Librusa.",
            "No timetable was included in the Librus response."
        ))
    })?;
    let root = root.get("Timetable").unwrap_or(root);
    if root.is_null() || root.as_array().is_some_and(Vec::is_empty) {
        return Ok(vec![]);
    }
    let Some(days) = root.as_object() else {
        bail!(tr!(
            "Nieoczekiwany format planu lekcji.",
            "Unexpected timetable format."
        ));
    };
    let mut output = vec![];
    fn visit(
        value: &Value,
        date: &str,
        number: usize,
        subjects: &HashMap<String, String>,
        teachers: &HashMap<String, String>,
        rooms: &HashMap<String, String>,
        out: &mut Vec<Lesson>,
    ) {
        if let Some(items) = value.as_array() {
            for item in items {
                visit(item, date, number, subjects, teachers, rooms, out);
            }
        } else if value.is_object()
            && (value.get("Subject").is_some() || value.get("Lesson").is_some())
        {
            let no = text(value, "LessonNo");
            let mut lesson = Lesson {
                date: date.into(),
                number: if no.is_empty() {
                    number.to_string()
                } else {
                    no
                },
                subject: resolve(&value["Subject"], subjects, tr!("Przedmiot", "Subject")),
                teacher: resolve(&value["Teacher"], teachers, tr!("Nauczyciel", "Teacher")),
                room: classroom(value, rooms),
                from: text(value, "HourFrom"),
                to: text(value, "HourTo"),
                canceled: flag(&value["IsCanceled"]),
                substitution: flag(&value["IsSubstitutionClass"]),
                note: text(value, "SubstitutionNote"),
                original: None,
            };
            if !lesson.canceled {
                lesson.original = original_lesson(value, &lesson, subjects, teachers, rooms);
            }
            out.push(lesson);
        }
    }
    for (date, slots) in days {
        if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() {
            if let Some(slots) = slots.as_array() {
                for (number, slot) in slots.iter().enumerate() {
                    visit(slot, date, number, subjects, teachers, rooms, &mut output);
                }
            }
        }
    }
    output.sort_by(|a, b| (&a.date, &a.from, &a.number).cmp(&(&b.date, &b.from, &b.number)));
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn sample(value: &str, weight: Option<f64>, semester: &str) -> Grade {
        Grade {
            id: String::new(),
            subject: "Matematyka".into(),
            value: value.into(),
            category: String::new(),
            date: String::new(),
            teacher: String::new(),
            semester: semester.into(),
            numeric: grade_number(value, 0.5, 0.25, false),
            counts: true,
            weight,
        }
    }
    #[test]
    fn room_symbols_ids_and_scalar_rooms_are_preserved() {
        let rooms = HashMap::from([("12".into(), "B-204".into())]);
        assert_eq!(
            classroom(&json!({"Classroom":{"Symbol":"A-103"}}), &rooms),
            "A-103"
        );
        assert_eq!(
            classroom(&json!({"Classroom":{"Symbol":"A-103","Id":12}}), &rooms),
            "B-204"
        );
        assert_eq!(classroom(&json!({"Classroom":{"Id":12}}), &rooms), "B-204");
        assert_eq!(classroom(&json!({"Classroom":"Gym"}), &rooms), "Gym");
        assert_eq!(classroom(&json!({}), &rooms), "—");
    }
    #[test]
    fn full_classroom_names_take_priority_over_abbreviations_and_numbers() {
        let full_name = "Pracownia informatyczna — budynek A, II piętro";
        let rooms = classroom_names(&[json!({"Id":12,"Name":full_name,"Symbol":"12","Number":12})]);
        let payload = json!({"Timetable":{"2026-11-20":[
            [{"Subject":{"Name":"Informatyka"},"Classroom":{"Name":full_name,"Symbol":"12"}}],
            [{"Subject":{"Name":"Matematyka"},"Classroom":{"Id":12}}],
            [{"Subject":{"Name":"Fizyka"},"Classroom":{"Id":12,"Symbol":"12"}}],
            [{"Subject":{"Name":"Plastyka"},"Classroom":"Pracownia plastyczna","Lesson":{"Classroom":{"Id":12}}}],
            [{"Subject":{"Name":"Chemia"},"ClassRoom":"Pracownia chemiczna / laboratorium 2"}],
            [{"Subject":{"Name":"WF"},"Room":{"Name":"Sala gimnastyczna","Number":3}}],
            [{"Subject":{"Name":"Technika"},"Lesson":{"Classroom":{"Name":"Pracownia techniczna","Symbol":"T-1"}}}]
        ]}});
        let lessons = timetable(&payload, &HashMap::new(), &HashMap::new(), &rooms).unwrap();
        assert_eq!(
            lessons
                .iter()
                .map(|lesson| lesson.room.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Pracownia informatyczna — budynek A, II piętro",
                "Pracownia informatyczna — budynek A, II piętro",
                "Pracownia informatyczna — budynek A, II piętro",
                "Pracownia plastyczna",
                "Pracownia chemiczna / laboratorium 2",
                "Sala gimnastyczna",
                "Pracownia techniczna",
            ]
        );
    }
    #[test]
    fn room_lookup_does_not_turn_available_rooms_into_blank_text() {
        let rooms = classroom_names(&[
            json!({"Id":12,"Symbol":"  ","Name":"B-204"}),
            json!({"Id":"13","Symbol":null,"Name":"","Number":103}),
            json!({"Id":14,"Symbol":"","Name":""}),
        ]);
        assert_eq!(rooms.get("12").map(String::as_str), Some("B-204"));
        assert_eq!(rooms.get("13").map(String::as_str), Some("103"));
        assert!(!rooms.contains_key("14"));
        let payload = json!({"Timetable":{"2026-11-10":[
            [{"Subject":{"Name":"Math"},"Classroom":{"Id":"12"}}],
            [{"Subject":{"Name":"Biology"},"Classroom":{"Id":13}}]
        ]}});
        let lessons = timetable(&payload, &HashMap::new(), &HashMap::new(), &rooms).unwrap();
        assert_eq!(lessons[0].room, "B-204");
        assert_eq!(lessons[1].room, "103");
        let blank_lookup = HashMap::from([("14".into(), " ".into())]);
        assert_eq!(
            classroom(&json!({"Classroom":{"Id":14}}), &blank_lookup),
            "14"
        );
    }
    #[test]
    fn room_text_survives_placeholders_and_nested_lesson_records() {
        assert_eq!(
            classroom(
                &json!({
                    "Classroom":{"Id":"missing"},"Room":{"Number":207}
                }),
                &HashMap::new()
            ),
            "207"
        );
        assert_eq!(
            classroom(
                &json!({
                    "Classroom":null,"Lesson":{"Classroom":{"Symbol":"Gym"}}
                }),
                &HashMap::new()
            ),
            "Gym"
        );
    }
    #[test]
    fn previous_grade_snapshots_without_server_ids_still_load() {
        let mut serialized = serde_json::to_value(sample("5", Some(1.), "1")).unwrap();
        serialized.as_object_mut().unwrap().remove("id");
        let grade: Grade = serde_json::from_value(serialized).unwrap();
        assert!(grade.id.is_empty());
        assert_eq!(grade.value, "5");
    }
    #[test]
    fn arithmetic_and_weighted_averages_respect_flags_semesters_and_unknown_weights() {
        let mut grades = vec![
            sample("4+", Some(3.), "1"),
            sample("6", Some(1.), "1"),
            sample("np", Some(5.), "1"),
            sample("2", Some(1.), "2"),
        ];
        let mut excluded = sample("1", Some(10.), "1");
        excluded.counts = false;
        grades.push(excluded);
        assert_eq!(grade_average(&grades, 1, false), Some(5.25));
        assert_eq!(grade_average(&grades, 1, true), Some(4.875));
        assert_eq!(grade_average(&grades, 2, true), Some(2.));
        assert_eq!(grade_number("4−", 0.5, 0.25, false), Some(3.75));
        assert_eq!(grade_number("0", 0.5, 0.25, false), None);
        assert_eq!(grade_number("85", 0.5, 0.25, false), None);
        grades[0].weight = None;
        assert_eq!(grade_average(&grades, 1, true), None);
        assert_eq!(grade_average(&[] as &[Grade], 0, false), None);
    }
    #[test]
    fn dates_follow_iso_weeks() {
        assert_eq!(
            monday(NaiveDate::from_ymd_opt(2026, 11, 15).unwrap()).to_string(),
            "2026-11-09"
        );
    }
    #[test]
    fn message_decoding_unwraps_xml_and_drops_executable_markup() {
        let raw=STANDARD.encode("<Message><Content><![CDATA[<p>Zażółć &amp; gęślą</p><script>bad()</script><p>Dziękuję</p>]]></Content></Message>");
        assert_eq!(decode_message(&raw), "Zażółć & gęślą\nDziękuję");
        assert_eq!(decode_message("zwykła wiadomość"), "zwykła wiadomość");
    }
    #[test]
    fn timetable_preserves_parallel_lessons_and_cancellations() {
        let payload = json!({"Timetable":{"2026-11-10":[[],[{"Subject":{"Id":"5"},"HourFrom":"08:00","IsCanceled":"1"},{"Subject":{"Name":"Biologia"},"IsSubstitutionClass":true}]]}});
        let names = HashMap::from([("5".into(), "Matematyka".into())]);
        let lessons = timetable(&payload, &names, &HashMap::new(), &HashMap::new()).unwrap();
        assert_eq!(lessons.len(), 2);
        assert!(lessons
            .iter()
            .any(|l| l.subject == "Matematyka" && l.canceled));
        assert!(lessons.iter().any(|l| l.substitution));
        assert!(timetable(&json!({"wrong":[]}), &names, &names, &names).is_err());
    }

    #[test]
    fn timetable_resolves_original_substitution_details_and_reads_old_cache() {
        let payload = json!({"Timetable":{"2026-11-10":[[{
            "LessonNo": "3", "HourFrom": "10:00", "HourTo": "10:45",
            "Subject": {"Name":"Biologia"}, "IsSubstitutionClass": true,
            "OrgSubject": {"Id":"5"}, "OrgTeacher": {"Id":"6"}, "OrgClassroom": {"Id":"7"},
            "OrgDate": "2026-11-09", "OrgLessonNo": 2, "OrgHourFrom": "09:00", "OrgHourTo": "09:45"
        }]]}});
        let subjects = HashMap::from([("5".into(), "Matematyka".into())]);
        let teachers = HashMap::from([("6".into(), "Anna Przykładowa".into())]);
        let rooms = HashMap::from([("7".into(), "Pracownia techniczna".into())]);
        let lessons = timetable(&payload, &subjects, &teachers, &rooms).unwrap();
        let original = lessons[0].original.as_ref().unwrap();
        assert_eq!(original.subject, "Matematyka");
        assert_eq!(original.teacher, "Anna Przykładowa");
        assert_eq!(original.room, "Pracownia techniczna");
        assert_eq!(original.date, "2026-11-09");
        assert_eq!(original.number, "2");
        assert_eq!(original.from, "09:00");
        assert_eq!(original.to, "09:45");

        let mut cached = serde_json::to_value(&lessons[0]).unwrap();
        cached.as_object_mut().unwrap().remove("original");
        assert!(serde_json::from_value::<Lesson>(cached)
            .unwrap()
            .original
            .is_none());
    }
}
