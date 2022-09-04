use chrono::Utc;

pub fn format_time() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
