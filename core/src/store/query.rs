use crate::store::query::QueryType::{EQ, GT, GTE, LIKE, LT, LTE, NE, CONTAINS, LIMIT, SORT};
use crate::store::types::StoreError;
use super::value::Value;

pub trait QueryBuilder: Send {
    type Output;

    fn on_eq(&mut self, field: &str, value: Value);
    fn on_ne(&mut self, field: &str, value: Value);
    fn on_gt(&mut self, field: &str, value: Value);
    fn on_gte(&mut self, field: &str, value: Value);
    fn on_lt(&mut self, field: &str, value: Value);
    fn on_lte(&mut self, field: &str, value: Value);

    fn begin_and(&mut self);
    fn begin_or(&mut self);
    fn end_group(&mut self);

    fn on_limit(&mut self, value: Value);
    fn on_sort(&mut self, field: &str, value: Value);

    fn on_like(&mut self, field: &str, value: Value);
    fn on_contains(&mut self, field: &str, value: Value);

    fn build(self) -> Self::Output;
}

#[derive(Copy, Clone, PartialEq)]
enum QueryType {
    EQ,
    NE,
    GT,
    GTE,
    LT,
    LTE,
    LIKE,
    CONTAINS,
    LIMIT,
    SORT,
}

impl QueryType {
    fn into_query(&self) -> &'static str {
        match self {
            EQ => "_eq",
            NE => "_ne",
            GT => "_gt",
            GTE => "_gte",
            LT => "_lt",
            LTE => "_lte",
            LIKE => "_like",
            CONTAINS => "_contains",
            LIMIT => "limit",
            SORT => "sort",
        }
    }
}

pub fn parse<B: QueryBuilder>(query: &str, mut builder: B) -> Result<B::Output, StoreError> {
    for q in query.split("&") {
        let split = q.split("=").collect::<Vec<&str>>();
        if split.len() != 2 {
            return Err(StoreError::InvalidQuery(q.to_string()))
        }

        let attribute_and_query = split[0];
        let value = Value::from_str(split[1]);

        match id_query(attribute_and_query)? {
            (att, NE) => builder.on_ne(&att, value),
            (att, GTE) => builder.on_gte(&att, value),
            (att, LTE) => builder.on_lte(&att, value),
            (att, EQ) => builder.on_eq(&att, value),
            (att, GT) => builder.on_gt(&att, value),
            (att, LT) => builder.on_lt(&att, value),
            (att, LIKE) => builder.on_like(&att, value),
            (att, CONTAINS) => builder.on_contains(&att, value),
            (_, LIMIT) => builder.on_limit(value),
            (att, SORT) => builder.on_sort(&att, value),
        };
    }

    Ok(builder.build())
}

const EXACT_ORDER: &[QueryType] = &[SORT, LIMIT];
const SUFFIX_ORDER: &[QueryType] = &[NE, GTE, LTE, EQ, GT, LT, LIKE, CONTAINS];

fn id_query(q: &str) -> Result<(String, QueryType), StoreError> {
    let q = q.trim().to_lowercase();
    for e_q in EXACT_ORDER {
        if q.id_is(*e_q) {
            return q.as_query_type(*e_q)
        }
    }
    for e_q in SUFFIX_ORDER {
        if q.id_ends_with(*e_q) {
            return q.as_query_type(*e_q)
        }
    }
    Ok((q.to_string(), EQ))
}

trait QueryParseFn {
    fn id_is(&self, q: QueryType) -> bool;
    fn id_ends_with(&self, q: QueryType) -> bool;
    fn as_query_type(&self, q: QueryType) -> Result<(String, QueryType), StoreError>;
}

impl QueryParseFn for String {
    fn id_is(&self, q: QueryType) -> bool {
       self == q.into_query()
    }

    fn id_ends_with(&self, q: QueryType) -> bool {
        self.ends_with(q.into_query())
    }

    fn as_query_type(&self, q: QueryType) -> Result<(String, QueryType), StoreError> {
        let attribute =  if EXACT_ORDER.contains(&q) {
            self
        } else {
            self.strip_suffix(q.into_query()).expect("impossible")
        };

        Ok((attribute.to_string(), q))
    }
}