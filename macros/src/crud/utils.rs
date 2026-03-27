pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

pub fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with('x') || s.ends_with("ch") || s.ends_with("sh") {
        format!("{}es", s)
    } else if s.ends_with('y')
        && !s.ends_with("ey")
        && !s.ends_with("ay")
        && !s.ends_with("oy")
        && !s.ends_with("uy")
    {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}
