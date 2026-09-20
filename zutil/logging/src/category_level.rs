pub fn level_for(category: &str) -> i64 {
    match category {
        "startup" => 0,
        "ui" => 0,
        "db" => 1,
        "parse" => 2,
        "critical" => 3,
        _ => 0,
    }
}
