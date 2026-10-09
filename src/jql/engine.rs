use std::cmp::Ordering;

use crate::{
    jql::{
        ast::{JqlAstNode, JqlBinaryKind},
        error::{JqlError, JqlErrorKind},
        proxy::Proxy,
    },
    json::{
        error::JsonError,
        value::{JsonLogic, JsonOrd, JsonValue},
    },
    source::Span,
};

#[derive(Clone)]
pub struct Engine {}

impl Engine {
    pub fn execute<'a>(value: Proxy<'a>, path: &'a JqlAstNode<'_>) -> Result<Proxy<'a>, JqlError> {
        match path {
            JqlAstNode::Null
            | JqlAstNode::Boolean(_)
            | JqlAstNode::Number(_)
            | JqlAstNode::String(_) => Ok(Self::literal(path)),
            JqlAstNode::Access(path) => Self::access(value, path),
            JqlAstNode::Pipe { source, dest } => Self::pipe(value, source, dest),
            JqlAstNode::Call { name, args } => Self::call(name, args, value),
            JqlAstNode::Binary { kind, left, right } => Self::binary(kind, left, right, value),
            _ => Err(op_not_support("Op is not supported")),
        }
    }

    fn literal<'a>(literal: &'a JqlAstNode<'a>) -> Proxy<'a> {
        match literal {
            JqlAstNode::Null => Proxy::owned(JsonValue::Null),
            JqlAstNode::Boolean(val) => Proxy::owned(JsonValue::boolean(*val)),
            JqlAstNode::String(st) => Proxy::owned(JsonValue::string(st.to_string())),
            JqlAstNode::Number(val) => Proxy::owned(JsonValue::number(*val)),
            _ => unreachable!(),
        }
    }

    fn access<'a>(value: Proxy<'a>, path: &'a str) -> Result<Proxy<'a>, JqlError> {
        value.get(path)
    }

    fn pipe<'a>(
        value: Proxy<'a>,
        source: &'a JqlAstNode<'a>,
        dest: &'a JqlAstNode<'a>,
    ) -> Result<Proxy<'a>, JqlError> {
        Self::execute(value, source).and_then(|r| Self::execute(r, dest))
    }

    fn call<'a>(
        name: &str,
        args: &'a Vec<JqlAstNode<'a>>,
        value: Proxy<'a>,
    ) -> Result<Proxy<'a>, JqlError> {
        match name {
            "filter" => Self::filter(value, &args[0]),
            _ => Err(op_not_support("Op is not supported")),
        }
    }

    pub fn binary<'a>(
        kind: &'a JqlBinaryKind,
        left: &'a JqlAstNode<'a>,
        right: &'a JqlAstNode<'a>,
        value: Proxy<'a>,
    ) -> Result<Proxy<'a>, JqlError> {
        let left_proxy = Engine::execute(value.clone(), left)?;
        let right_proxy = Engine::execute(value.clone(), right)?;
        let left_output = left_proxy.data();
        let right_output = right_proxy.data();
        let result = match kind {
            JqlBinaryKind::And => {
                left_output.try_as_bool().map_err(json_exec_error)?
                    && right_output.try_as_bool().map_err(json_exec_error)?
            }
            JqlBinaryKind::Or => {
                left_output.try_as_bool().map_err(json_exec_error)?
                    || right_output.try_as_bool().map_err(json_exec_error)?
            }
            JqlBinaryKind::Equal => left_output == right_output,
            JqlBinaryKind::NotEqual => left_output != right_output,
            JqlBinaryKind::Greater => {
                left_output.try_cmp(right_output).map_err(json_exec_error)? == Ordering::Greater
            }
            JqlBinaryKind::Less => {
                left_output.try_cmp(right_output).map_err(json_exec_error)? == Ordering::Less
            }
            JqlBinaryKind::GreaterEqual => {
                left_output.try_cmp(right_output).map_err(json_exec_error)? != Ordering::Less
            }
            JqlBinaryKind::LessEqual => {
                left_output.try_cmp(right_output).map_err(json_exec_error)? != Ordering::Greater
            }
        };
        Ok(Proxy::owned(JsonValue::boolean(result)))
    }

    fn filter<'a>(value: Proxy<'a>, predictive: &'a JqlAstNode) -> Result<Proxy<'a>, JqlError> {
        let data = value.data();
        match data {
            JsonValue::Array(items) => {
                let filtered = items
                    .clone()
                    .iter()
                    .filter(
                        |x| match Engine::execute(Proxy::new(x), &predictive.clone()) {
                            Ok(p) => matches!(p.data(), JsonValue::True),
                            err => {
                                println!("err = {:?}", err);
                                false
                            }
                        },
                    )
                    .cloned()
                    .collect::<Vec<JsonValue>>();
                Ok(Proxy::owned(JsonValue::array(filtered)))
            }
            _ => Err(JqlError::new(
                JqlErrorKind::ExecutionError,
                Span { start: 0, end: 0 },
            )),
        }
    }
}

fn json_exec_error(_: JsonError) -> JqlError {
    JqlError::from_kind(JqlErrorKind::ExecutionError)
}

fn op_not_support(_: &str) -> JqlError {
    JqlError::from_kind(JqlErrorKind::ExecutionError)
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::jql::error::{JqlError, JqlErrorKind};
    use crate::jql::jql_parse;
    use crate::jql::proxy::Proxy;
    use crate::json::value::JsonValue;

    fn fixture() -> JsonValue {
        JsonValue::object([
            (
                "matrix".to_string(),
                JsonValue::array(vec![
                    JsonValue::array(vec![
                        JsonValue::number(1.0),
                        JsonValue::number(2.0),
                        JsonValue::number(3.0),
                    ]),
                    JsonValue::array(vec![
                        JsonValue::number(4.0),
                        JsonValue::number(5.0),
                        JsonValue::number(6.0),
                    ]),
                ]),
            ),
            (
                "nested".to_string(),
                JsonValue::array(vec![
                    JsonValue::number(1.0),
                    JsonValue::array(vec![
                        JsonValue::number(2.0),
                        JsonValue::array(vec![JsonValue::number(3.0)]),
                    ]),
                ]),
            ),
            (
                "jobs".to_string(),
                JsonValue::array(vec![
                    JsonValue::object([(
                        "title".to_string(),
                        JsonValue::string("Dev".to_string()),
                    )]),
                    JsonValue::object([("title".to_string(), JsonValue::string("QA".to_string()))]),
                ]),
            ),
            ("count".to_string(), JsonValue::number(42.0)),
        ])
    }

    fn execute(data: &JsonValue, query: &str) -> Result<JsonValue, JqlError> {
        let ast = jql_parse(query)?;
        Engine::execute(Proxy::new(data), &ast).map(|p| p.data().clone())
    }

    fn assert_ok(data: &JsonValue, query: &str, expected: JsonValue) {
        let actual =
            execute(data, query).unwrap_or_else(|e| panic!("query {query:?} should succeed: {e}"));
        assert_eq!(actual, expected, "query {query:?}");
    }

    fn assert_err(data: &JsonValue, query: &str) {
        assert!(execute(data, query).is_err(), "query {query:?} should fail");
    }

    fn assert_exec_err(data: &JsonValue, query: &str) {
        let err = execute(data, query).expect_err(&format!("query {query:?} should fail"));
        assert_eq!(
            err.kind,
            JqlErrorKind::ExecutionError,
            "query {query:?} should be an execution error, got {err}"
        );
    }

    fn bool_cases(data: &JsonValue, cases: &[(&str, bool)]) {
        for &(query, expected) in cases {
            assert_ok(data, query, JsonValue::boolean(expected));
        }
    }

    #[test]
    fn access_root() {
        let data = fixture();
        assert_ok(&data, ".", data.clone());
    }

    #[test]
    fn access_property() {
        let data = fixture();
        assert_ok(&data, ".count", JsonValue::number(42.0));
    }

    #[test]
    fn access_array_index() {
        let data = fixture();
        assert_ok(
            &data,
            ".jobs[1]",
            JsonValue::object([("title".to_string(), JsonValue::string("QA".to_string()))]),
        );
    }

    #[test]
    fn access_nested_array_via_dotted_index() {
        // Nested indexing uses empty-name property: .a[i].[j]
        let data = fixture();
        assert_ok(&data, ".matrix[0].[1]", JsonValue::number(2.0));
        assert_ok(&data, ".matrix[1].[0]", JsonValue::number(4.0));
        assert_ok(&data, ".nested[1].[0]", JsonValue::number(2.0));
    }

    #[test]
    fn access_deeply_nested_array_via_dotted_index() {
        let data = fixture();
        assert_ok(&data, ".nested[1].[1].[0]", JsonValue::number(3.0));
        assert_ok(
            &data,
            ".matrix[0].[1:3]",
            JsonValue::array(vec![JsonValue::number(2.0), JsonValue::number(3.0)]),
        );
        assert_ok(
            &data,
            ".matrix[1].[0,2]",
            JsonValue::array(vec![JsonValue::number(4.0), JsonValue::number(6.0)]),
        );
    }

    #[test]
    fn access_root_nested_array_via_dotted_index() {
        let data = JsonValue::array(vec![
            JsonValue::array(vec![JsonValue::number(10.0), JsonValue::number(20.0)]),
            JsonValue::array(vec![JsonValue::number(30.0), JsonValue::number(40.0)]),
        ]);
        assert_ok(&data, ".[0].[1]", JsonValue::number(20.0));
        assert_ok(&data, ".[1].[0]", JsonValue::number(30.0));
    }

    #[test]
    fn access_nested_array_then_property() {
        let data = JsonValue::object([(
            "rows".to_string(),
            JsonValue::array(vec![JsonValue::array(vec![JsonValue::object([(
                "name".to_string(),
                JsonValue::string("a".to_string()),
            )])])]),
        )]);
        assert_ok(
            &data,
            ".rows[0].[0].name",
            JsonValue::string("a".to_string()),
        );
    }

    #[test]
    fn consecutive_brackets_without_dot_are_accepted() {
        let data = fixture();
        assert_ok(&data, ".matrix[0][1]", JsonValue::number(2.0));
        assert_ok(&data, ".nested[1][0]", JsonValue::number(2.0));

        let root = JsonValue::array(vec![
            JsonValue::array(vec![JsonValue::number(10.0), JsonValue::number(20.0)]),
            JsonValue::array(vec![JsonValue::number(30.0), JsonValue::number(40.0)]),
        ]);
        assert_ok(&root, ".[0][1]", JsonValue::number(20.0));
    }

    #[test]
    fn pipe_chains_access() {
        let data = fixture();
        assert_ok(&data, ".matrix[0] | .[1]", JsonValue::number(2.0));
        assert_ok(&data, ".nested[1] | .[1] | .[0]", JsonValue::number(3.0));
        assert_ok(
            &data,
            ".jobs | .[0].title",
            JsonValue::string("Dev".to_string()),
        );
    }

    #[test]
    fn missing_property_is_execution_error() {
        let data = fixture();
        let err = execute(&data, ".missing").expect_err("should fail");
        assert_eq!(err.kind, JqlErrorKind::ExecutionError);
    }

    #[test]
    fn literals_return_json_values() {
        let data = fixture();
        assert_ok(&data, "null", JsonValue::Null);
        assert_ok(&data, "true", JsonValue::True);
        assert_ok(&data, "false", JsonValue::False);
        assert_ok(&data, "42", JsonValue::number(42.0));
        assert_ok(&data, "\"hi\"", JsonValue::string("hi".to_string()));
    }

    #[test]
    fn unsupported_call_is_execution_error() {
        let data = fixture();
        let err = execute(&data, "unknown(.count)").expect_err("unknown call should fail");
        assert_eq!(err.kind, JqlErrorKind::ExecutionError);
    }

    #[test]
    fn compares_numbers_from_literals_and_fields() {
        let data = fixture();
        bool_cases(
            &data,
            &[
                ("1 < 2", true),
                ("2 < 1", false),
                ("2 > 1", true),
                ("1 > 2", false),
                ("2 <= 2", true),
                ("2 <= 1", false),
                ("2 >= 2", true),
                ("1 >= 2", false),
                (".count > 41", true),
                (".count > 42", false),
                (".count < 43", true),
                (".count < 42", false),
                (".count >= 42", true),
                (".count <= 42", true),
                (".count >= 43", false),
                (".count <= 41", false),
                (".matrix[0].[0] < .matrix[0].[1]", true),
                (".matrix[1].[0] > .matrix[0].[2]", true),
            ],
        );
    }

    #[test]
    fn equality_is_structural_across_json_values() {
        let data = JsonValue::object([
            ("n".to_string(), JsonValue::number(1.0)),
            ("s".to_string(), JsonValue::string("x".to_string())),
            ("flag".to_string(), JsonValue::True),
            (
                "xs".to_string(),
                JsonValue::array(vec![JsonValue::number(1.0), JsonValue::number(2.0)]),
            ),
            (
                "ys".to_string(),
                JsonValue::array(vec![JsonValue::number(1.0), JsonValue::number(2.0)]),
            ),
            (
                "zs".to_string(),
                JsonValue::array(vec![JsonValue::number(1.0)]),
            ),
            (
                "obj".to_string(),
                JsonValue::object([("k".to_string(), JsonValue::string("v".to_string()))]),
            ),
            (
                "obj2".to_string(),
                JsonValue::object([("k".to_string(), JsonValue::string("v".to_string()))]),
            ),
            (
                "obj3".to_string(),
                JsonValue::object([("k".to_string(), JsonValue::string("w".to_string()))]),
            ),
        ]);
        bool_cases(
            &data,
            &[
                ("1 == 1", true),
                ("1 == 2", false),
                ("1 != 2", true),
                ("true == true", true),
                ("true == false", false),
                ("false != true", true),
                ("null == null", true),
                ("null == false", false),
                ("\"x\" == \"x\"", true),
                ("\"x\" == \"y\"", false),
                ("\"x\" != \"y\"", true),
                (".n == 1", true),
                (".s == \"x\"", true),
                (".flag == true", true),
                (".n == true", false),
                (".s == 1", false),
                (".flag == null", false),
                (".xs == .ys", true),
                (".xs != .zs", true),
                (".obj == .obj2", true),
                (".obj != .obj3", true),
                (".xs == .obj", false),
            ],
        );
    }

    #[test]
    fn logical_ops_combine_boolean_results() {
        let data = fixture();
        bool_cases(
            &data,
            &[
                ("true && true", true),
                ("true && false", false),
                ("false && true", false),
                ("false && false", false),
                ("true || true", true),
                ("true || false", true),
                ("false || true", true),
                ("false || false", false),
                (".count > 1 && .count < 100", true),
                (".count > 100 && .count == 42", false),
                (".count > 100 || .count == 42", true),
                (".count < 1 || .count > 100", false),
                ("(.count > 1 && .count < 100) || false", true),
                ("false || (.count == 42 && true)", true),
                ("true && false || true", true),
                ("false || false && true", false),
            ],
        );
    }

    #[test]
    fn comparisons_bind_tighter_than_logical_ops_at_runtime() {
        let data = fixture();
        // 1 > 2 && 3 < 4  => false && true => false
        assert_ok(&data, "1 > 2 && 3 < 4", JsonValue::False);
        // 1 < 2 || 3 > 4 => true || false => true
        assert_ok(&data, "1 < 2 || 3 > 4", JsonValue::True);
        // Parentheses override the default shape.
        assert_ok(&data, "(1 > 2 || 3 < 4) && true", JsonValue::True);
        assert_ok(&data, "1 > 2 || (3 < 4 && false)", JsonValue::False);
    }

    #[test]
    fn binary_ops_work_after_pipe() {
        let data = fixture();
        assert_ok(&data, ".count | . > 40", JsonValue::True);
        assert_ok(&data, ".count | . == 42", JsonValue::True);
        assert_ok(&data, ".jobs[0].title | . == \"Dev\"", JsonValue::True);
        assert_ok(&data, ".matrix[0] | .[1] > 1 && .[1] < 3", JsonValue::True);
    }

    #[test]
    fn ordered_compare_fails_when_either_side_is_not_a_number() {
        let data = fixture();
        for query in [
            "true > false",
            "false < true",
            "\"a\" < \"b\"",
            "\"a\" >= \"a\"",
            "null <= 1",
            "1 <= null",
            "true >= 0",
            ".jobs > 1",
            "1 < .jobs",
            ".jobs[0].title > \"A\"",
            "true <= .count",
            ".count >= false",
        ] {
            assert_exec_err(&data, query);
        }
    }

    #[test]
    fn logical_ops_fail_when_a_used_operand_is_not_boolean() {
        let data = fixture();
        for query in [
            "true && 1",
            "1 && true",
            "false || 0",
            "null || false",
            "true && null",
            "\"x\" && true",
            "\"x\" || false",
            ".count && true",
            "false || .count",
            ".jobs || false",
            "true && .jobs[0]",
        ] {
            assert_exec_err(&data, query);
        }
    }

    #[test]
    fn binary_ops_evaluate_both_operands() {
        let data = fixture();
        // Even when the left side alone would decide a boolean result, a
        // failing right operand still fails the whole expression.
        assert_exec_err(&data, "true || .missing");
        assert_exec_err(&data, "false && .missing");
        assert_exec_err(&data, "1 < 2 && .missing");
        assert_exec_err(&data, ".missing || true");
        assert_exec_err(&data, ".missing && false");
        assert_exec_err(&data, ".missing == 1");
        assert_exec_err(&data, "1 == .missing");
        assert_exec_err(&data, ".missing < 1");
        assert_exec_err(&data, "1 < .missing");
    }

    #[test]
    fn invalid_access_path_fails() {
        let data = fixture();
        assert_err(&data, ".jobs[");
        assert_err(&data, ".matrix[0].[");
    }
}
