fn main() {
    println!(
        "{}",
        librus_mobile_core::dispatch(serde_json::json!({"op":"demo","week":"2026-11-09"})).unwrap()
    );
}
