// Fictional, memory-only fixtures for visual QA. Never used by a live account.
use crate::data::*;
use chrono::{Duration, NaiveDate};
use std::collections::HashMap;
pub fn payloads(week: NaiveDate) -> HashMap<Section, Payload> {
    let subjects = [
        "Matematyka",
        "Język polski",
        "Język angielski",
        "Biologia",
        "Historia",
    ];
    let mut grades = vec![];
    for (i, subject) in subjects.iter().enumerate() {
        for (j, value) in ["5", "4+", "5−", "6"].iter().enumerate() {
            grades.push(Grade {
                subject: (*subject).into(),
                value: (*value).into(),
                category: ["Sprawdzian", "Kartkówka", "Aktywność", "Projekt"][j].into(),
                date: (week - Duration::days((i + j) as i64)).to_string(),
                teacher: "Anna Przykładowa".into(),
                semester: "1".into(),
                numeric: grade_number(value, 0.5, 0.25, false),
                counts: true,
                weight: Some((j % 3 + 1) as f64),
            });
        }
    }
    grades.sort_by(|a, b| b.date.cmp(&a.date));
    let attendance = (0..40)
        .map(|i| Attendance {
            date: (week - Duration::days(i / 6)).to_string(),
            lesson: (i % 6 + 1).to_string(),
            subject: subjects[i as usize % 5].into(),
            kind: if i % 8 == 0 {
                "Nieobecność usprawiedliwiona"
            } else {
                "Obecność"
            }
            .into(),
            present: Some(i % 8 != 0),
        })
        .collect();
    let notices=vec![Notice{id:"demo-notice-1".into(),title:"Dzień otwarty — spotkajmy się w szkole".into(),content:"Zapraszamy uczniów i rodziców na dzień otwarty.\n\nW programie warsztaty przedmiotowe, spotkania z wychowawcami i prezentacje prac uczniów. Szczegółowy harmonogram będzie dostępny przy wejściu.\n\nDo zobaczenia!".into(),date:week.to_string(),until:(week+Duration::days(7)).to_string(),author:"Sekretariat szkoły".into(),unread:true},Notice{id:"demo-notice-2".into(),title:"Biblioteka: nowe książki już na półkach".into(),content:"Nowości literackie i książki popularnonaukowe czekają na czytelników. Zapraszamy w godzinach pracy biblioteki.".into(),date:(week-Duration::days(2)).to_string(),until:String::new(),author:"Biblioteka szkolna".into(),unread:false}];
    let mut lessons = vec![];
    for day in 0..5 {
        for hour in 0..6 {
            lessons.push(Lesson {
                date: (week + Duration::days(day)).to_string(),
                number: (hour + 1).to_string(),
                subject: subjects[(day + hour) as usize % 5].into(),
                teacher: ["Anna Przykładowa", "Jan Przykładowy"][(hour % 2) as usize].into(),
                room: (101 + hour).to_string(),
                from: format!("{:02}:00", 8 + hour),
                to: format!("{:02}:45", 8 + hour),
                canceled: day == 2 && hour == 3,
                substitution: day == 1 && hour == 2,
                note: if day == 1 && hour == 2 {
                    "Zmiana sali na 103".into()
                } else {
                    String::new()
                },
            });
        }
    }
    let messages=vec![Message{id:"demo-message-1".into(),title:"Wycieczka klasowa — informacje organizacyjne".into(),sender:"Anna Przykładowa".into(),content:"Dzień dobry,\n\nprzypominam o naszej wycieczce klasowej. Spotykamy się przed szkołą o godzinie 8:00. Proszę zabrać wygodne buty, wodę i dobry humor.\n\nDokładny plan wyjazdu omówimy na godzinie wychowawczej.\n\nPozdrawiam serdecznie,\nAnna Przykładowa".into(),date:format!("{week} 14:30"),unread:true,attachments:vec![],has_files:false},Message{id:"demo-message-2".into(),title:"Przypomnienie o projekcie z biologii".into(),sender:"Jan Przykładowy".into(),content:"Przypominam o terminie oddania projektu. Tematy znajdziecie w materiałach z lekcji.".into(),date:format!("{week} 09:15"),unread:false,attachments:vec![],has_files:false}];
    HashMap::from([
        (
            Section::Grades,
            Payload::Grades {
                items: grades,
                behaviour: vec![BehaviourGrade {
                    value: "bdb".into(),
                    description: "Bardzo dobre".into(),
                    semester: "1".into(),
                    date: week.to_string(),
                }],
                behaviour_error: None,
            },
        ),
        (Section::Attendance, Payload::Attendance(attendance)),
        (Section::Notices, Payload::Notices(notices)),
        (Section::Timetable, Payload::Timetable(lessons)),
        (
            Section::Messages,
            Payload::Messages {
                total: messages.len(),
                items: messages,
            },
        ),
    ])
}
