use gearbox_rs_core::{QueryBuilder, Value, parse};

/// Records every call the parser makes so we can assert against them.
#[derive(Default)]
struct MockQueryBuilder {
    calls: Vec<(String, String, Option<Value>)>, // (method, field, value)
}

impl MockQueryBuilder {
    fn push(&mut self, method: &str, field: &str, value: Value) {
        self.calls.push((method.to_string(), field.to_string(), Some(value)));
    }
}

impl QueryBuilder for MockQueryBuilder {
    type Output = Vec<(String, String, Option<Value>)>;

    fn on_eq(&mut self, field: &str, value: Value) { self.push("eq", field, value); }
    fn on_ne(&mut self, field: &str, value: Value) { self.push("ne", field, value); }
    fn on_gt(&mut self, field: &str, value: Value) { self.push("gt", field, value); }
    fn on_gte(&mut self, field: &str, value: Value) { self.push("gte", field, value); }
    fn on_lt(&mut self, field: &str, value: Value) { self.push("lt", field, value); }
    fn on_lte(&mut self, field: &str, value: Value) { self.push("lte", field, value); }
    fn on_like(&mut self, field: &str, value: Value) { self.push("like", field, value); }
    fn on_contains(&mut self, field: &str, value: Value) { self.push("contains", field, value); }

    fn on_limit(&mut self, value: Value) { self.push("limit", "", value); }
    fn on_sort(&mut self, field: &str, value: Value) { self.push("sort", field, value); }

    fn begin_and(&mut self) { self.calls.push(("begin_and".to_string(), String::new(), None)); }
    fn begin_or(&mut self) { self.calls.push(("begin_or".to_string(), String::new(), None)); }
    fn end_group(&mut self) { self.calls.push(("end_group".to_string(), String::new(), None)); }

    fn build(self) -> Self::Output { self.calls }
}

// ── Basic operator dispatch ──

#[test]
fn test_eq_explicit() {
    let calls = parse("name_eq=john", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "eq");
    assert_eq!(calls[0].1, "name");
}

#[test]
fn test_eq_implicit_no_suffix() {
    let calls = parse("name=john", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "eq");
    assert_eq!(calls[0].1, "name");
}

#[test]
fn test_ne() {
    let calls = parse("status_ne=inactive", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "ne");
    assert_eq!(calls[0].1, "status");
}

#[test]
fn test_gt() {
    let calls = parse("age_gt=25", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "gt");
    assert_eq!(calls[0].1, "age");
}

#[test]
fn test_gte() {
    let calls = parse("age_gte=25", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "gte");
    assert_eq!(calls[0].1, "age");
}

#[test]
fn test_lt() {
    let calls = parse("price_lt=100", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "lt");
    assert_eq!(calls[0].1, "price");
}

#[test]
fn test_lte() {
    let calls = parse("price_lte=100", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "lte");
    assert_eq!(calls[0].1, "price");
}

#[test]
fn test_like() {
    let calls = parse("name_like=john", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "like");
    assert_eq!(calls[0].1, "name");
}

#[test]
fn test_contains() {
    let calls = parse("bio_contains=rust", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "contains");
    assert_eq!(calls[0].1, "bio");
}

// ── Limit and sort (exact matches) ──

#[test]
fn test_limit() {
    let calls = parse("limit=10", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "limit");
}

#[test]
fn test_sort() {
    let calls = parse("sort=-created_at", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "sort");
}

// ── Value type inference ──

#[test]
fn test_value_bool() {
    let calls = parse("active=true", MockQueryBuilder::default()).unwrap();
    assert!(matches!(calls[0].2, Some(Value::Bool(true))));
}

#[test]
fn test_value_int() {
    let calls = parse("age_gt=25", MockQueryBuilder::default()).unwrap();
    assert!(matches!(calls[0].2, Some(Value::Int(25))));
}

#[test]
fn test_value_float() {
    let calls = parse("price_lt=9.99", MockQueryBuilder::default()).unwrap();
    assert!(matches!(calls[0].2, Some(Value::Float(f)) if (f - 9.99).abs() < f64::EPSILON));
}

#[test]
fn test_value_string() {
    let calls = parse("name=john", MockQueryBuilder::default()).unwrap();
    assert!(matches!(&calls[0].2, Some(Value::String(s)) if s == "john"));
}

#[test]
fn test_value_null() {
    let calls = parse("deleted_at=null", MockQueryBuilder::default()).unwrap();
    assert!(matches!(calls[0].2, Some(Value::Null)));
}

// ── Multiple conditions ──

#[test]
fn test_multiple_conditions() {
    let calls = parse("name=john&age_gt=25&active=true", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, "eq");
    assert_eq!(calls[0].1, "name");
    assert_eq!(calls[1].0, "gt");
    assert_eq!(calls[1].1, "age");
    assert_eq!(calls[2].0, "eq");
    assert_eq!(calls[2].1, "active");
}

#[test]
fn test_conditions_with_limit_and_sort() {
    let calls = parse("name_like=john&limit=10&sort=-created_at", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, "like");
    assert_eq!(calls[1].0, "limit");
    assert_eq!(calls[2].0, "sort");
}

// ── Edge cases ──

#[test]
fn test_field_with_underscores() {
    let calls = parse("created_at_gte=2024-01-01", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "gte");
    assert_eq!(calls[0].1, "created_at");
}

#[test]
fn test_field_named_rate_limit_not_confused_with_limit() {
    let calls = parse("rate_limit_eq=100", MockQueryBuilder::default()).unwrap();
    assert_eq!(calls[0].0, "eq");
    assert_eq!(calls[0].1, "rate_limit");
}

#[test]
fn test_invalid_query_no_equals() {
    let result = parse("garbage", MockQueryBuilder::default());
    assert!(result.is_err());
}
