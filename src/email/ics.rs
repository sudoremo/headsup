use chrono::NaiveDate;
use uuid::Uuid;

/// Represents an ICS calendar event
pub struct IcsEvent {
    pub uid: String,
    pub sequence: u32,
    pub summary: String,
    pub description: String,
    pub date: NaiveDate,
    pub url: Option<String>,
}

impl IcsEvent {
    /// Generate a deterministic UID for a subject
    pub fn generate_uid(subject_id: Uuid) -> String {
        format!("headsup-{}@headsup", subject_id)
    }

    /// Render the event as an ICS (iCalendar) string
    pub fn to_ics(&self) -> String {
        let dtstamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let dtstart = self.date.format("%Y%m%d");

        let url_line = self
            .url
            .as_ref()
            .map(|u| format!("URL:{}\r\n", ics_escape(u)))
            .unwrap_or_default();

        format!(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             PRODID:-//Headsup//Headsup//EN\r\n\
             METHOD:PUBLISH\r\n\
             BEGIN:VEVENT\r\n\
             UID:{uid}\r\n\
             DTSTAMP:{dtstamp}\r\n\
             DTSTART;VALUE=DATE:{dtstart}\r\n\
             SUMMARY:{summary}\r\n\
             DESCRIPTION:{description}\r\n\
             SEQUENCE:{sequence}\r\n\
             {url_line}\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            uid = self.uid,
            dtstamp = dtstamp,
            dtstart = dtstart,
            summary = ics_escape(&self.summary),
            description = ics_escape(&self.description),
            sequence = self.sequence,
            url_line = url_line,
        )
    }
}

/// Parse an exact date string (YYYY-MM-DD) into a NaiveDate.
/// Returns None for any other format.
pub fn parse_exact_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

/// Escape special characters for ICS text fields
fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}
