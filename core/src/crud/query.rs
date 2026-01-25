//! Query building traits and types for CRUD operations.

/// Sort direction for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order (default).
    Asc,
    /// Descending order.
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        SortDirection::Asc
    }
}

/// Sort specification parsed from query parameters.
///
/// Supports the common pattern of using "-field" for descending order:
/// - `"name"` -> sort by name ASC
/// - `"-created_at"` -> sort by created_at DESC
#[derive(Debug, Clone)]
pub struct SortSpec {
    /// Column name to sort by.
    pub column: String,
    /// Sort direction.
    pub direction: SortDirection,
}

impl SortSpec {
    /// Create a new sort specification.
    pub fn new(column: impl Into<String>, direction: SortDirection) -> Self {
        SortSpec {
            column: column.into(),
            direction,
        }
    }

    /// Parse a sort string like "name" or "-created_at".
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if let Some(col) = s.strip_prefix('-') {
            SortSpec {
                column: col.to_string(),
                direction: SortDirection::Desc,
            }
        } else {
            SortSpec {
                column: s.to_string(),
                direction: SortDirection::Asc,
            }
        }
    }

    /// Generate the SQL ORDER BY clause fragment.
    pub fn to_sql(&self) -> String {
        match self.direction {
            SortDirection::Asc => self.column.clone(),
            SortDirection::Desc => format!("{} DESC", self.column),
        }
    }
}

/// Trait for query DTOs that can build SQL WHERE clauses.
///
/// This trait is automatically implemented for Query DTOs generated
/// by the `#[derive(Crud)]` macro.
pub trait BuildWhereClause {
    /// Build WHERE clause conditions and parameter values.
    ///
    /// Returns a tuple of:
    /// - `Vec<String>`: SQL condition fragments (e.g., "name = $1", "active = $2")
    /// - `Vec<String>`: Parameter values as strings (to be parsed/bound by the executor)
    ///
    /// The parameter placeholders use PostgreSQL-style `$N` notation.
    fn build_conditions(&self) -> (Vec<String>, Vec<String>);

    /// Get pagination parameters (limit, offset).
    fn pagination(&self) -> (Option<i64>, Option<i64>);

    /// Get the sort specification, if any.
    fn sort_spec(&self) -> Option<SortSpec>;

    /// Build a complete WHERE clause string.
    ///
    /// Returns `None` if there are no conditions, otherwise returns
    /// a string like "WHERE name = $1 AND active = $2".
    fn build_where_clause(&self) -> Option<String> {
        let (conditions, _) = self.build_conditions();
        if conditions.is_empty() {
            None
        } else {
            Some(format!("WHERE {}", conditions.join(" AND ")))
        }
    }

    /// Build ORDER BY clause, if sort is specified.
    fn build_order_by(&self) -> Option<String> {
        self.sort_spec().map(|spec| format!("ORDER BY {}", spec.to_sql()))
    }

    /// Build LIMIT clause, if specified.
    fn build_limit(&self) -> Option<String> {
        let (limit, _) = self.pagination();
        limit.map(|l| format!("LIMIT {}", l))
    }

    /// Build OFFSET clause, if specified.
    fn build_offset(&self) -> Option<String> {
        let (_, offset) = self.pagination();
        offset.map(|o| format!("OFFSET {}", o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_spec_parse_asc() {
        let spec = SortSpec::parse("name");
        assert_eq!(spec.column, "name");
        assert_eq!(spec.direction, SortDirection::Asc);
    }

    #[test]
    fn test_sort_spec_parse_desc() {
        let spec = SortSpec::parse("-created_at");
        assert_eq!(spec.column, "created_at");
        assert_eq!(spec.direction, SortDirection::Desc);
    }

    #[test]
    fn test_sort_spec_to_sql() {
        let asc = SortSpec::new("name", SortDirection::Asc);
        assert_eq!(asc.to_sql(), "name");

        let desc = SortSpec::new("created_at", SortDirection::Desc);
        assert_eq!(desc.to_sql(), "created_at DESC");
    }
}
