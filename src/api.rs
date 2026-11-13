use crate::data::{self, Attendance, Grade, Notice, Payload, Section};
use crate::i18n::{tr, trf};
use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use reqwest::{
    blocking::{Client, Response},
    cookie::{CookieStore, Jar},
    Url,
};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const AUTH: &str = "https://api.librus.pl/OAuth/Authorization?client_id=46";
const API: &str = "https://synergia.librus.pl/gateway/api/2.0";
const AUTH_SCOPE: &str =
    "https://api.librus.pl/OAuth/Authorization?client_id=46&response_type=code&scope=mydata";

fn message_url(id: &str) -> Result<Url> {
    let mut url = Url::parse("https://wiadomosci.librus.pl/api/inbox/messages/")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!(tr!("Nieprawidłowy adres wiadomości", "Invalid message URL")))?
        .pop_if_empty()
        .push(id);
    Ok(url)
}
fn grade_counts(value: &Value) -> bool {
    value.get("IsConstituent").is_none_or(data::flag)
        && ![
            "IsSemester",
            "IsFinal",
            "IsSemesterProposition",
            "IsFinalProposition",
        ]
        .iter()
        .any(|key| data::flag(&value[*key]))
}

fn collect_pages(
    start: Url,
    key: &str,
    mut fetch: impl FnMut(&Url) -> Result<Value>,
) -> Result<Vec<Value>> {
    let mut url = start.clone();
    let mut visited = HashSet::new();
    let mut items = Vec::new();
    for _ in 0..100 {
        if !visited.insert(url.as_str().to_owned()) {
            bail!(tr!(
                "Librus zwrócił zapętlone stronicowanie danych.",
                "Librus returned a pagination loop."
            ));
        }
        let payload = fetch(&url)?;
        items.extend(data::records(&payload, key)?.iter().cloned());
        let next = &payload["Pages"]["Next"];
        let next = match next {
            Value::Null | Value::Bool(false) => None,
            Value::String(value) => Some(value.as_str()),
            Value::Object(value) => Some(value.get("Url").and_then(Value::as_str).context(tr!(
                "Nieoczekiwany adres kolejnej strony Librusa",
                "Unexpected Librus next-page URL"
            ))?),
            _ => bail!(tr!(
                "Nieoczekiwany format stronicowania Librusa",
                "Unexpected Librus pagination format"
            )),
        };
        let Some(next) = next.filter(|s| !s.trim().is_empty()) else {
            return Ok(items);
        };
        let mut candidate = url.join(next).context(tr!(
            "Nieprawidłowy adres kolejnej strony Librusa",
            "Invalid Librus next-page URL"
        ))?;
        candidate.set_fragment(None);
        // Follow only server-advertised pages of this exact collection. Never
        // invent page/limit parameters or forward the cookie jar elsewhere.
        if candidate.origin() != start.origin()
            || candidate.path().trim_end_matches('/') != start.path().trim_end_matches('/')
            || !candidate.username().is_empty()
            || candidate.password().is_some()
        {
            bail!(tr!(
                "Librus zwrócił nieoczekiwany adres kolejnej strony danych.",
                "Librus returned an unexpected next-page URL."
            ));
        }
        url = candidate;
    }
    bail!(tr!(
        "Librus zwrócił zbyt wiele stron danych. Nie pokazano niepełnego zestawu.",
        "Librus returned too many pages. Incomplete data was not displayed."
    ))
}

pub struct Session {
    client: Mutex<Client>,
    lookups: Mutex<HashMap<&'static str, Vec<Value>>>,
    messages_ready: AtomicBool,
    credentials: crate::vault::Credentials,
    renewal: Mutex<()>,
    generation: AtomicU64,
    closed: AtomicBool,
    authenticated: AtomicBool,
}

#[derive(Debug)]
struct SessionExpired;
impl std::fmt::Display for SessionExpired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(tr!("Sesja Librusa wygasła i nie udało się jej odnowić. Zaloguj się ponownie; poprzedni zapis danych pozostał zachowany.", "Your Librus session expired and could not be renewed. Sign in again; your previous data snapshot was preserved."))
    }
}
impl std::error::Error for SessionExpired {}
fn retry_expired<T>(
    mut request: impl FnMut() -> Result<T>,
    renew: impl FnOnce() -> Result<()>,
) -> Result<T> {
    match request() {
        Err(error) if error.is::<SessionExpired>() => {
            renew()?;
            request()
        }
        result => result,
    }
}
fn new_client() -> Result<(Client, Arc<Jar>)> {
    let jar = Arc::new(Jar::default());
    let client=Client::builder().cookie_provider(jar.clone()).timeout(Duration::from_secs(25)).connect_timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::none()).build()?;
    Ok((client, jar))
}

fn trusted(url: &Url) -> bool {
    url.scheme() == "https"
        && matches!(url.host_str(), Some("api.librus.pl" | "synergia.librus.pl"))
        && url.port_or_known_default() == Some(443)
}

fn redirect_target(response: &Response) -> Result<Option<Url>> {
    response
        .headers()
        .get(reqwest::header::LOCATION)
        .map(|location| {
            response
                .url()
                .join(location.to_str()?)
                .context("Invalid login redirect")
        })
        .transpose()
}

fn prime_login(client: &Client, entry: &str, allowed: impl Fn(&Url) -> bool) -> Result<()> {
    let bootstrap = client
        .get(entry)
        .header("Referer", "https://portal.librus.pl/")
        .send()?
        .error_for_status()?;
    let authorization =
        redirect_target(&bootstrap)?.context("Librus did not return the initial OAuth redirect")?;
    if !allowed(&authorization) {
        bail!("Unexpected OAuth bootstrap domain");
    }
    client.get(authorization).send()?.error_for_status()?;
    Ok(())
}

fn action_message(actions: &Value) -> Option<&'static str> {
    let actions = actions.as_array()?;
    if actions.is_empty() {
        return None;
    }
    if actions.iter().any(|action| {
        matches!(
            action.get("code").and_then(Value::as_str),
            Some("leakedPasswordChangeAdvised" | "leakedPasswordChangeRequired")
        )
    }) {
        return Some("Librus reports that your password appears in known leaks. Open the official Synergia website, change your password, then sign in here with the new password.");
    }
    if actions
        .iter()
        .any(|action| action.get("code").and_then(Value::as_str) == Some("passwordChangeRequired"))
    {
        return Some("Librus requires a password change. Complete it on the official Synergia website, then sign in here with your new password.");
    }
    Some("Librus requires an account setup or verification step. Complete it on the official Synergia website, then sign in here again.")
}

// Librus can end PerformLogin with HTTP 200 and an HTML action screen,
// rather than a redirect. Read only its structured action codes; never display
// the page or execute its JavaScript.
fn html_action_message(html: &str) -> Option<&'static str> {
    let marker = html.find("var requiredActions")?;
    let (_, json) = html[marker..].split_once('=')?;
    let actions = serde_json::Deserializer::from_str(json.trim_start())
        .into_iter::<Value>()
        .next()?
        .ok()?;
    action_message(&actions)
}

// Explicitly process each response before the next request. The bootstrap must
// stop at the state-bearing authorization response, rather than following all
// its redirects before the credentials are submitted.
fn finish_redirects(
    client: &Client,
    mut url: Url,
    allowed: impl Fn(&Url) -> bool,
) -> Result<String> {
    let mut steps = Vec::new();
    for _ in 0..12 {
        // Librus's multi-domain handoff can advertise an HTTP message URL.
        // Upgrade the known destination before issuing any request so neither
        // session cookies nor the handoff query ever travel over plaintext HTTP.
        url = secure_message_redirect(url)?;
        if !allowed(&url) {
            bail!(
                "Unexpected login redirect destination: {}://{}:{}",
                url.scheme(),
                url.host_str().unwrap_or("unknown"),
                url.port_or_known_default().unwrap_or(0)
            );
        }
        let response = client
            .get(url)
            .send()
            .context("Could not complete login redirect")?;
        let status = response.status();
        // Paths and statuses only: never retain query strings, cookies or tokens.
        steps.push(format!("{} {}", response.url().path(), status.as_u16()));
        if matches!(status.as_u16(), 401 | 403) {
            return Err(SessionExpired.into());
        }
        if status.is_client_error() || status.is_server_error() {
            bail!("Login redirect failed ({})", steps.join(" → "));
        }
        let next = redirect_target(&response)?;
        match next {
            Some(next) => url = next,
            None if status.is_redirection() => {
                bail!("Login redirect did not contain a destination")
            }
            None => {
                let body = response
                    .text()
                    .context("Could not read the final login step")?;
                if let Some(message) = html_action_message(&body) {
                    bail!("{message}");
                }
                return Ok(steps.join(" → "));
            }
        }
    }
    bail!("Too many login redirects");
}

fn secure_message_redirect(mut url: Url) -> Result<Url> {
    if url.host_str() == Some("wiadomosci.librus.pl")
        && url.scheme() == "http"
        && url.port_or_known_default() == Some(80)
    {
        url.set_scheme("https").map_err(|_| {
            anyhow::anyhow!(tr!(
                "Nieprawidłowy adres modułu wiadomości",
                "Invalid messaging module URL"
            ))
        })?;
        url.set_port(None).map_err(|_| {
            anyhow::anyhow!(tr!(
                "Nieprawidłowy port modułu wiadomości",
                "Invalid messaging module port"
            ))
        })?;
    }
    Ok(url)
}

fn has_session_cookie(jar: &Jar) -> bool {
    jar.cookies(&Url::parse(&format!("{API}/Auth/TokenInfo/")).unwrap())
        .and_then(|header| {
            header.to_str().ok().map(|text| {
                text.split(';')
                    .any(|cookie| cookie.trim().starts_with("oauth_token="))
            })
        })
        .unwrap_or(false)
}

impl Session {
    fn client(&self) -> Client {
        self.client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::Acquire)
    }
    pub fn deferred(login: &str, password: &str) -> Result<Self> {
        let (client, _) = new_client()?;
        Ok(Self {
            client: Mutex::new(client),
            lookups: Mutex::new(HashMap::new()),
            messages_ready: AtomicBool::new(false),
            credentials: crate::vault::Credentials {
                login: login.to_owned(),
                password: password.to_owned(),
            },
            renewal: Mutex::new(()),
            generation: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            authenticated: AtomicBool::new(false),
        })
    }
    fn renew(&self, generation: u64) -> Result<()> {
        let _guard = self.renewal.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed.load(Ordering::Acquire) {
            bail!(tr!(
                "Konto zostało wylogowane.",
                "The account has been signed out."
            ));
        }
        if self.generation.load(Ordering::Acquire) != generation {
            return Ok(());
        }
        let client = self.client();
        let refreshed = (|| -> Result<bool> {
            let response = client
                .get("https://synergia.librus.pl/refreshToken")
                .send()?;
            if !response.status().is_success() {
                return Ok(false);
            }
            let response = client.get(format!("{API}/Auth/TokenInfo/")).send()?;
            Ok(response.status().is_success() && response.json::<Value>().is_ok())
        })()
        .unwrap_or(false);
        if !refreshed {
            let fresh =
                Self::login(&self.credentials.login, &self.credentials.password).context(tr!(
                "Nie udało się automatycznie odnowić logowania. Poprzedni zapis został zachowany",
                "Could not renew the session automatically. Your previous snapshot was preserved"
            ))?;
            *self.client.lock().unwrap_or_else(|e| e.into_inner()) = fresh.client();
        }
        self.messages_ready.store(false, Ordering::Release);
        self.authenticated.store(true, Ordering::Release);
        self.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
    pub fn login(login: &str, password: &str) -> Result<Self> {
        let (client, jar) = new_client()?;
        // Preserve the state-bearing API response and its cookies. Do not walk
        // the remainder of this chain until the login form has been posted.
        prime_login(
            &client,
            "https://synergia.librus.pl/loguj/portalRodzina",
            trusted,
        )?;
        let response = client
            .post(AUTH)
            .header("Accept", "application/json")
            .header("Origin", "https://api.librus.pl")
            .header("Referer", AUTH)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Accept-Language", "pl-PL,pl;q=0.9,en;q=0.8")
            .header("Cache-Control", "no-cache")
            .form(&[("action", "login"), ("login", login), ("pass", password)])
            .send()
            .context("Could not contact Librus")?;
        let status = response.status();
        let body: Value = response.json().context(
            "Librus returned an unexpected login page. A browser challenge may be required.",
        )?;
        if let Some(message) = body.get("actions").and_then(action_message) {
            bail!("{message}");
        }
        if !status.is_success() || body.get("goTo").and_then(Value::as_str).is_none() {
            // Never display raw login responses: they can contain tokens or personal data.
            let message = body.to_string().to_lowercase();
            if message.contains("robot") || message.contains("captcha") {
                bail!("Librus requires CAPTCHA. Complete login on the official website and try again; this app cannot solve CAPTCHA.");
            }
            if message.contains("hasło") || message.contains("login") {
                bail!("Login was rejected. Check your Synergia username and password.");
            }
            bail!("Librus did not complete login. Your account may require an additional verification step.");
        }
        let next = Url::parse(AUTH_SCOPE)?.join(body["goTo"].as_str().unwrap())?;
        let trace = finish_redirects(&client, next, trusted)?;
        if !has_session_cookie(&jar) {
            bail!("Librus accepted the login response but did not issue the Synergia session cookie. Additional verification may be required. Login steps: {trace}");
        }
        let session = Self {
            client: Mutex::new(client),
            credentials: crate::vault::Credentials {
                login: login.to_owned(),
                password: password.to_owned(),
            },
            renewal: Mutex::new(()),
            generation: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            authenticated: AtomicBool::new(true),
            lookups: Mutex::new(HashMap::new()),
            messages_ready: AtomicBool::new(false),
        };
        // Use the canonical endpoint including its trailing slash. During login,
        // distinguish an API rejection from an already-expired user session.
        let verification = session
            .client()
            .get(format!("{API}/Auth/TokenInfo/"))
            .header("Accept", "application/json")
            .header("Referer", "https://synergia.librus.pl/")
            .send()?;
        if !verification.status().is_success() {
            bail!("Librus issued a session cookie, but session verification returned HTTP {}. Login steps: {trace}", verification.status().as_u16());
        }
        verification
            .json::<Value>()
            .context("Session verification returned an unexpected response")?;
        Ok(session)
    }

    fn get_url(&self, url: &str) -> Result<Value> {
        let generation = self.generation.load(Ordering::Acquire);
        retry_expired(
            || {
                if self.closed.load(Ordering::Acquire) {
                    bail!(tr!(
                        "Konto zostało wylogowane.",
                        "The account has been signed out."
                    ));
                }
                if url.starts_with("https://wiadomosci.librus.pl/api/") {
                    self.prepare_messages()?;
                }
                self.request_json(url)
            },
            || self.renew(generation),
        )
    }
    fn request_json(&self, url: &str) -> Result<Value> {
        let response = self
            .client()
            .get(url)
            .header("Accept", "application/json")
            .send()?;
        if response.status().is_redirection() {
            return Err(SessionExpired.into());
        }
        if matches!(response.status().as_u16(), 401 | 403) {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            if text.to_ascii_lowercase().contains("insufficient scopes") {
                bail!(tr!(
                    "Konto nie ma uprawnień do tych danych.",
                    "This account does not have permission to access this data."
                ));
            }
            if status.as_u16() == 403
                && self
                    .client()
                    .get(format!("{API}/Auth/TokenInfo/"))
                    .send()
                    .is_ok_and(|r| r.status().is_success())
            {
                bail!(tr!(
                    "Librus odmówił dostępu do tych danych (HTTP 403).",
                    "Librus denied access to this data (HTTP 403)."
                ));
            }
            self.authenticated.store(false, Ordering::Release);
            return Err(SessionExpired.into());
        }
        response
            .error_for_status()?
            .json()
            .context("Librus returned an unexpected API response")
    }

    fn get(&self, path: &str) -> Result<Value> {
        self.get_url(&format!("{API}/{path}"))
    }

    fn collection(&self, path: &str, key: &str) -> Result<Vec<Value>> {
        collect_pages(Url::parse(&format!("{API}/{path}"))?, key, |url| {
            self.get_url(url.as_str())
        })
    }

    fn lookup(&self, path: &'static str, key: &str) -> Vec<Value> {
        if let Ok(cache) = self.lookups.lock() {
            if let Some(items) = cache.get(path) {
                return items.clone();
            }
        }
        match self.collection(path, key) {
            Ok(items) => {
                if let Ok(mut cache) = self.lookups.lock() {
                    cache.insert(path, items.clone());
                }
                items
            }
            Err(_) => Vec::new(), // Missing optional descriptions fall back to explicit IDs.
        }
    }

    pub fn load(&self, section: Section, week: NaiveDate, page: usize) -> Result<Payload> {
        match section {
            Section::Grades => {
                let grades = self.collection("Grades", "Grades")?;
                let subjects = data::names(&self.lookup("Subjects", "Subjects"));
                let category_records = self
                    .collection("Grades/Categories", "Categories")
                    .unwrap_or_default();
                let categories = data::names(&category_records);
                let teachers = data::names(&self.lookup("Users", "Users"));
                let grading = self.get("GradingSystem").unwrap_or(Value::Null);
                let number = |key: &str, fallback: f64| {
                    grading[key]
                        .as_f64()
                        .or_else(|| {
                            grading[key]
                                .as_str()
                                .and_then(|s| s.replace(',', ".").parse().ok())
                        })
                        .filter(|n| n.is_finite())
                        .map(f64::abs)
                        .unwrap_or(fallback)
                };
                let mut rows: Vec<_> = grades
                    .iter()
                    .map(|v| {
                        let category = category_records
                            .iter()
                            .find(|c| data::text(c, "Id") == data::text(&v["Category"], "Id"));
                        Grade {
                            subject: data::resolve(
                                &v["Subject"],
                                &subjects,
                                tr!("Przedmiot", "Subject"),
                            ),
                            value: data::text(v, "Grade"),
                            category: data::resolve(
                                &v["Category"],
                                &categories,
                                tr!("Kategoria", "Category"),
                            ),
                            date: data::text(v, "Date"),
                            teacher: data::resolve(
                                &v["AddedBy"],
                                &teachers,
                                tr!("Nauczyciel", "Teacher"),
                            ),
                            semester: data::text(v, "Semester"),
                            numeric: data::grade_number(
                                &data::text(v, "Grade"),
                                number("plusValue", 0.5),
                                number("minusValue", 0.25),
                                data::flag(&grading["countZero"]),
                            ),
                            counts: grade_counts(v),
                            weight: category.map(|c| {
                                c["Weight"]
                                    .as_f64()
                                    .or_else(|| c["Weight"].as_str().and_then(|s| s.parse().ok()))
                                    .filter(|n| n.is_finite() && *n >= 0.)
                                    .unwrap_or(1.)
                            }),
                        }
                    })
                    .collect();
                rows.sort_by(|a, b| b.date.cmp(&a.date));
                let mut from_subject = Vec::new();
                rows.retain(|row| {
                    if row.subject.trim().to_lowercase() == "zachowanie" {
                        from_subject.push(data::BehaviourGrade {
                            value: row.value.clone(),
                            description: row.category.clone(),
                            semester: row.semester.clone(),
                            date: row.date.clone(),
                        });
                        false
                    } else {
                        true
                    }
                });
                let behaviour = self.collection("BehaviourGrades", "Grades").map(|items| {
                    items
                        .iter()
                        .map(|v| {
                            let kind = data::text(&v["BehaviourGrade"], "Id");
                            let canonical = match kind.as_str() {
                                "1" => "wz",
                                "2" => "bdb",
                                "3" => "db",
                                "4" => "popr",
                                "5" => "ndp",
                                "6" => "ng",
                                _ => "",
                            };
                            let value = [
                                canonical.to_owned(),
                                data::text(v, "ShortName"),
                                data::text(v, "Value"),
                                data::text(v, "Text"),
                            ]
                            .into_iter()
                            .find(|s| !s.is_empty())
                            .unwrap_or_else(|| "—".into());
                            data::BehaviourGrade {
                                value,
                                description: data::text(v, "Text"),
                                semester: data::text(v, "Semester"),
                                date: data::text(v, "AddDate"),
                            }
                        })
                        .collect::<Vec<_>>()
                });
                let (mut behaviour,behaviour_error)=match behaviour{Ok(items)=>(items,None),Err(_)=>(vec![],Some(tr!("Nie udało się odświeżyć zachowania. Pozostawiono poprzedni zapis, jeśli jest dostępny.", "Could not refresh behavior grades. The previous snapshot was kept, if available.").into()))};
                behaviour.extend(from_subject);
                Ok(Payload::Grades {
                    items: rows,
                    behaviour,
                    behaviour_error,
                })
            }
            Section::Attendance => {
                let records = self.collection("Attendances", "Attendances")?;
                let types = self.collection("Attendances/Types", "Types")?;
                let lessons = self.lookup("Lessons", "Lessons");
                let subjects = data::names(&self.lookup("Subjects", "Subjects"));
                let mut rows: Vec<_> = records
                    .iter()
                    .map(|v| {
                        let type_id = data::text(&v["Type"], "Id");
                        let kind = types.iter().find(|t| data::text(t, "Id") == type_id);
                        let lesson_id = data::text(&v["Lesson"], "Id");
                        let subject = lessons
                            .iter()
                            .find(|l| data::text(l, "Id") == lesson_id)
                            .map(|l| {
                                data::resolve(&l["Subject"], &subjects, tr!("Przedmiot", "Subject"))
                            })
                            .unwrap_or_else(|| "—".into());
                        Attendance {
                            date: data::text(v, "Date"),
                            lesson: data::text(v, "LessonNo"),
                            subject,
                            kind: kind
                                .map(|t| data::text(t, "Name"))
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| trf!("Typ #{type_id}", "Type #{type_id}")),
                            present: kind.and_then(|t| t.get("IsPresenceKind")).map(data::flag),
                        }
                    })
                    .collect();
                rows.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.lesson.cmp(&b.lesson)));
                Ok(Payload::Attendance(rows))
            }
            Section::Notices => {
                let records = self.collection("SchoolNotices", "SchoolNotices")?;
                let teachers = data::names(&self.lookup("Users", "Users"));
                let mut rows: Vec<_> = records
                    .iter()
                    .map(|v| Notice {
                        id: data::text(v, "Id"),
                        title: data::plain_html(&data::text(v, "Subject")),
                        content: data::plain_html(&data::text(v, "Content")),
                        date: data::text(v, "CreationDate"),
                        until: data::text(v, "EndDate"),
                        author: data::resolve(&v["AddedBy"], &teachers, tr!("Szkoła", "School")),
                        unread: !data::flag(&v["WasRead"]),
                    })
                    .collect();
                rows.sort_by(|a, b| b.date.cmp(&a.date));
                Ok(Payload::Notices(rows))
            }
            Section::Timetable => {
                let value = self.get(&format!("Timetables?weekStart={week}"))?;
                let subjects = data::names(&self.lookup("Subjects", "Subjects"));
                let teachers = data::names(&self.lookup("Users", "Users"));
                let records = self.lookup("Classrooms", "Classrooms");
                let rooms = records
                    .iter()
                    .map(|r| {
                        let symbol = data::text(r, "Symbol");
                        (
                            data::text(r, "Id"),
                            if symbol.is_empty() {
                                data::text(r, "Name")
                            } else {
                                symbol
                            },
                        )
                    })
                    .collect();
                Ok(Payload::Timetable(data::timetable(
                    &value, &subjects, &teachers, &rooms,
                )?))
            }
            Section::Messages => {
                let value = self.get_url(&format!(
                    "https://wiadomosci.librus.pl/api/inbox/messages?page={page}&limit=20"
                ))?;
                let items = data::records(&value, "data")?
                    .iter()
                    .map(|v| data::message(v, false))
                    .collect::<Vec<_>>();
                let total = value["total"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .or_else(|| value["total"].as_str().and_then(|s| s.parse().ok()))
                    .unwrap_or_else(|| {
                        (page - 1) * 20 + items.len() + usize::from(items.len() == 20)
                    });
                Ok(Payload::Messages { items, total })
            }
            Section::Account => bail!(tr!(
                "Zakładka konta nie pobiera danych.",
                "The profile tab does not download data."
            )),
        }
    }

    fn prepare_messages(&self) -> Result<()> {
        if self.messages_ready.load(Ordering::Acquire) {
            return Ok(());
        }
        finish_redirects(
            &self.client(),
            Url::parse("https://synergia.librus.pl/wiadomosci3")?,
            |u| {
                trusted(u)
                    || (u.scheme() == "https"
                        && u.host_str() == Some("wiadomosci.librus.pl")
                        && u.port_or_known_default() == Some(443))
            },
        )
        .context(tr!(
            "Nie udało się otworzyć modułu wiadomości",
            "Could not open the messaging module"
        ))?;
        self.messages_ready.store(true, Ordering::Release);
        Ok(())
    }

    pub fn message_detail(&self, id: &str) -> Result<data::Message> {
        let url = message_url(id)?;
        let value = self.get_url(url.as_str())?;
        let item = value.get("data").filter(|v| v.is_object()).context(tr!(
            "Librus zwrócił nieoczekiwany format wiadomości",
            "Librus returned an unexpected message format"
        ))?;
        Ok(data::message(item, true))
    }

    pub fn logout(&self) -> Result<()> {
        self.closed.store(true, Ordering::Release);
        self.authenticated.store(false, Ordering::Release);
        let _guard = self.renewal.lock().unwrap_or_else(|e| e.into_inner());
        self.client()
            .get("https://synergia.librus.pl/wyloguj")
            .send()?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn local_all_grade_average_is_not_disabled_by_category_display_settings() {
        let value = serde_json::json!({"IsConstituent":true,"Category":{"CountToTheAverage":false,"Weight":null},"IsFinal":false});
        assert!(grade_counts(&value));
        assert!(!grade_counts(
            &serde_json::json!({"IsConstituent":true,"IsFinal":true})
        ));
    }
    #[test]
    fn message_handoff_is_upgraded_before_sending_any_request() {
        let url = secure_message_redirect(
            Url::parse("http://wiadomosci.librus.pl:80/MultiDomainLogon?code=fictional").unwrap(),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://wiadomosci.librus.pl/MultiDomainLogon?code=fictional"
        );
        let other = Url::parse("http://other.example/path").unwrap();
        assert_eq!(secure_message_redirect(other.clone()).unwrap(), other);
    }
    #[test]
    fn expired_requests_renew_only_on_demand_and_retry_once() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let renewals = Cell::new(0);
        let result = retry_expired(
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    Err(SessionExpired.into())
                } else {
                    Ok(42)
                }
            },
            || {
                renewals.set(renewals.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, 42);
        assert_eq!(calls.get(), 2);
        assert_eq!(renewals.get(), 1);
        calls.set(0);
        renewals.set(0);
        let result: Result<()> = retry_expired(
            || {
                calls.set(calls.get() + 1);
                Err(SessionExpired.into())
            },
            || {
                renewals.set(renewals.get() + 1);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 2);
        assert_eq!(renewals.get(), 1);
        renewals.set(0);
        let result: Result<()> = retry_expired(
            || bail!("HTTP 500"),
            || {
                renewals.set(renewals.get() + 1);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(renewals.get(), 0);
    }
    #[test]
    fn preparing_a_remembered_session_does_not_log_in() {
        let session = Session::deferred("fictional-account", "fictional-password").unwrap();
        assert!(!session.is_authenticated());
        assert_eq!(session.generation.load(Ordering::Acquire), 0);
    }

    #[test]
    fn gateway_collections_start_without_pagination_parameters() {
        for endpoint in [
            "Grades",
            "SchoolNotices",
            "Attendances",
            "Attendances/Types",
            "Subjects",
            "Lessons",
            "Users",
            "Grades/Categories",
            "Classrooms",
        ] {
            let url = Url::parse(&format!("{API}/{endpoint}")).unwrap();
            let items = collect_pages(url.clone(), "Items", |request| {
                assert_eq!(request, &url);
                assert_eq!(
                    request.query(),
                    None,
                    "Gateway endpoint must not receive invented query parameters"
                );
                Ok(serde_json::json!({"Items":[{"Id":1}]}))
            })
            .unwrap();
            assert_eq!(items.len(), 1);
        }
    }

    #[test]
    fn gateway_follows_only_advertised_next_pages() {
        let url = Url::parse(&format!("{API}/Grades")).unwrap();
        let mut requests = 0;
        let items=collect_pages(url,"Grades",|request|{
            requests+=1;
            match requests {
                1=>{assert_eq!(request.query(),None);Ok(serde_json::json!({"Grades":[{"Id":1}],"Pages":{"Next":{"Url":"?cursor=server-chosen"}}}))},
                2=>{assert_eq!(request.query(),Some("cursor=server-chosen"));Ok(serde_json::json!({"Grades":[{"Id":2}],"Pages":{"Next":format!("{API}/Grades/?cursor=final")}}))},
                3=>{assert_eq!(request.path(),"/gateway/api/2.0/Grades/");assert_eq!(request.query(),Some("cursor=final"));Ok(serde_json::json!({"Grades":[{"Id":3}],"Pages":{"Next":""}}))},
                _=>panic!("Unexpected extra request"),
            }
        }).unwrap();
        assert_eq!(requests, 3);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn gateway_rejects_untrusted_or_looping_pagination() {
        let url = Url::parse(&format!("{API}/Grades")).unwrap();
        for next in ["https://other.example/Grades", "/wyloguj", "?repeat=1"] {
            let mut requests = 0;
            let result = collect_pages(url.clone(), "Grades", |_| {
                requests += 1;
                Ok(serde_json::json!({"Grades":[],"Pages":{"Next":next}}))
            });
            assert!(result.is_err());
            assert!(requests <= 2);
        }
        assert!(
            collect_pages(url, "Grades", |_| Ok(serde_json::json!({"unexpected":[]}))).is_err()
        );
    }

    #[test]
    fn message_ids_form_one_path_segment_without_double_slashes() {
        assert_eq!(
            message_url("sample-id").unwrap().as_str(),
            "https://wiadomosci.librus.pl/api/inbox/messages/sample-id"
        );
        let url = message_url("a/b?secret=x").unwrap();
        assert_eq!(url.host_str(), Some("wiadomosci.librus.pl"));
        assert_eq!(url.query(), None);
        assert!(url.path().contains("a%2Fb%3Fsecret=x"));
    }

    #[test]
    #[ignore = "Requires the user's remembered account in Windows Credential Manager; reads lists only"]
    fn live_account_read_only() -> Result<()> {
        let credentials =
            crate::vault::load()?.context("Brak zapamiętanego konta do testu integracyjnego")?;
        let session = Session::login(&credentials.login, &credentials.password).map_err(|_| {
            anyhow::anyhow!("Logowanie testowe nie powiodło się; dane konta nie są wypisywane")
        })?;
        let week = data::monday(data::today());
        let mut failed = false;
        for section in [
            Section::Grades,
            Section::Attendance,
            Section::Messages,
            Section::Notices,
            Section::Timetable,
        ] {
            match session.load(section, week, 1) {
                Ok(payload) => {
                    println!(
                        "{}: OK, {} wpisów (treści ukryte)",
                        section.title(),
                        payload.len()
                    );
                    if let Payload::Grades {
                        items,
                        behaviour_error,
                        ..
                    } = &payload
                    {
                        println!("Średnia arytmetyczna dostępna: {}, ważona dostępna: {}, zachowanie pobrane: {}",data::grade_average(items,0,false).is_some(),data::grade_average(items,0,true).is_some(),behaviour_error.is_none());
                        if behaviour_error.is_some() {
                            failed = true;
                        }
                    }
                }
                Err(error) => {
                    println!("{}: błąd pobierania lub formatu", section.title());
                    for cause in error.chain() {
                        if let Some(http) = cause.downcast_ref::<reqwest::Error>() {
                            println!("HTTP status: {:?}", http.status().map(|s| s.as_u16()));
                        } else {
                            println!(
                                "Przyczyna: {}",
                                cause
                                    .to_string()
                                    .split("https://")
                                    .next()
                                    .unwrap_or("błąd sieci")
                                    .split('?')
                                    .next()
                                    .unwrap_or("błąd")
                            );
                        }
                    }
                    failed = true;
                }
            }
        }
        session
            .renew(session.generation.load(Ordering::Acquire))
            .map_err(|_| anyhow::anyhow!("Odnowienie sesji testowej nie powiodło się"))?;
        session
            .get("Grades")
            .map_err(|_| anyhow::anyhow!("Żądanie po odnowieniu sesji nie powiodło się"))?;
        println!("Odnowienie sesji i ponowne pobranie: OK");
        if failed {
            bail!("Część testów integracyjnych nie przeszła");
        }
        Ok(())
    }

    #[test]
    fn identifies_password_warning_in_successful_html_response() {
        let html = r#"<script>
            var requiredActions = [{"code":"leakedPasswordChangeAdvised","message":"unused","additionalData":{}}]
            ShowNextStepForm(requiredActions)
        </script>"#;
        let message = html_action_message(html).unwrap();
        assert!(message.contains("known leaks"));
        assert!(message.contains("official Synergia"));
        for code in [
            "passwordChangeRequired",
            "leakedPasswordChangeRequired",
            "unknownFutureAction",
        ] {
            assert!(html_action_message(&format!(
                "var requiredActions = [{{\"code\":\"{code}\"}}];"
            ))
            .is_some());
        }
        assert!(html_action_message("var requiredActions = []; next();").is_none());
        assert!(html_action_message("var requiredActions = invalid;").is_none());
        assert!(html_action_message("<html>normal page</html>").is_none());
    }

    #[test]
    fn identifies_json_action_required_response() {
        let actions = serde_json::json!([{"code": "passwordChangeRequired"}]);
        assert!(action_message(&actions)
            .unwrap()
            .contains("requires a password change"));
    }

    #[test]
    fn bootstrap_stops_before_login_and_redirects_keep_cookies() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            // Every request is checked in order: following /premature would
            // consume the OAuth bootstrap before the login POST, the regression.
            let steps = [
                (
                    "GET /start ",
                    "",
                    "302 Found",
                    "Location: /auth?state=private\r\nSet-Cookie: oauth_state=state; Path=/\r\n",
                ),
                (
                    "GET /auth?state=private ",
                    "oauth_state=state",
                    "302 Found",
                    "Location: /premature\r\nSet-Cookie: api_session=api; Path=/\r\n",
                ),
                ("POST /auth ", "api_session=api", "200 OK", ""),
                (
                    "GET /perform ",
                    "oauth_state=state",
                    "302 Found",
                    "Location: /grant?code=private\r\nSet-Cookie: flow=ready; Path=/\r\n",
                ),
                (
                    "GET /grant?code=private ",
                    "flow=ready",
                    "302 Found",
                    "Location: /callback?code=private\r\n",
                ),
                (
                    "GET /callback?code=private ",
                    "api_session=api",
                    "200 OK",
                    "Set-Cookie: oauth_token=token; HttpOnly; Path=/\r\n",
                ),
            ];
            for (path, cookie, status, headers) in steps {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut byte = [0u8];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                let text = String::from_utf8(request).unwrap();
                assert!(
                    text.starts_with(path),
                    "Unexpected request: {}",
                    text.lines().next().unwrap()
                );
                assert!(text.contains(cookie));
                let content_length = text
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; content_length];
                stream.read_exact(&mut body).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            }
        });
        let jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(jar.clone())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let allowed = |url: &Url| url.host_str() == Some("127.0.0.1");
        prime_login(&client, &format!("{base}/start"), allowed).unwrap();
        client
            .post(format!("{base}/auth"))
            .form(&[("action", "login")])
            .send()
            .unwrap();
        let trace = finish_redirects(
            &client,
            Url::parse(&format!("{base}/perform")).unwrap(),
            allowed,
        )
        .unwrap();
        assert!(trace.contains("/callback 200"));
        assert!(!trace.contains("private"));
        assert!(!trace.contains("token"));
        assert!(jar
            .cookies(&Url::parse(&base).unwrap())
            .unwrap()
            .to_str()
            .unwrap()
            .contains("oauth_token=token"));
        server.join().unwrap();
    }

    #[test]
    fn a_session_cookie_must_apply_to_the_api_path() {
        let jar = Jar::default();
        let url = Url::parse("https://synergia.librus.pl/").unwrap();
        jar.add_cookie_str("oauth_token=wrong_path; Secure; Path=/other", &url);
        assert!(!has_session_cookie(&jar));
        jar.add_cookie_str("oauth_token=valid; Secure; HttpOnly; Path=/", &url);
        assert!(has_session_cookie(&jar));
    }

    #[test]
    fn redirects_stay_on_librus() {
        for url in [
            "http://api.librus.pl/",
            "https://api.librus.pl.evil.test/",
            "https://evil.test/",
            "https://api.librus.pl:444/",
        ] {
            assert!(!trusted(&Url::parse(url).unwrap()));
        }
        assert!(trusted(&Url::parse(AUTH).unwrap()));
        assert!(trusted(
            &Url::parse("https://synergia.librus.pl/loguj").unwrap()
        ));
    }
}
