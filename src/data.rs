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
            Self::Grades => "Oceny",
            Self::Attendance => "Frekwencja",
            Self::Messages => "Wiadomości",
            Self::Notices => "Ogłoszenia",
            Self::Timetable => "Plan lekcji",
            Self::Account => "Twoje konto",
        }
    }
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Grades => "Oceny z przedmiotów i podsumowanie średnich",
            Self::Attendance => "Obecności, nieobecności i historia wpisów",
            Self::Messages => "Skrzynka odbiorcza i treść wiadomości",
            Self::Notices => "Informacje i komunikaty od szkoły",
            Self::Timetable => "Twój tydzień i zmiany w planie",
            Self::Account => "Logowanie i ustawienia wyglądu",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grade {
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
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Librus zwrócił nieoczekiwany format danych ({key})."))
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
                            .unwrap_or_else(|| "Załącznik".into())
                    })
                    .collect()
            })
            .unwrap_or_default(),
        has_files: flag(&value["isAnyFileAttached"]),
    }
}

pub fn timetable(
    value: &Value,
    subjects: &HashMap<String, String>,
    teachers: &HashMap<String, String>,
    rooms: &HashMap<String, String>,
) -> Result<Vec<Lesson>> {
    let root = value
        .get("Timetable")
        .ok_or_else(|| anyhow::anyhow!("Brak planu w odpowiedzi Librusa."))?;
    let root = root.get("Timetable").unwrap_or(root);
    if root.is_null() || root.as_array().is_some_and(Vec::is_empty) {
        return Ok(vec![]);
    }
    let Some(days) = root.as_object() else {
        bail!("Nieoczekiwany format planu lekcji.");
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
            out.push(Lesson {
                date: date.into(),
                number: if no.is_empty() {
                    number.to_string()
                } else {
                    no
                },
                subject: resolve(&value["Subject"], subjects, "Przedmiot"),
                teacher: resolve(&value["Teacher"], teachers, "Nauczyciel"),
                room: resolve(&value["Classroom"], rooms, "Sala"),
                from: text(value, "HourFrom"),
                to: text(value, "HourTo"),
                canceled: flag(&value["IsCanceled"]),
                substitution: flag(&value["IsSubstitutionClass"]),
                note: text(value, "SubstitutionNote"),
            });
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
}
