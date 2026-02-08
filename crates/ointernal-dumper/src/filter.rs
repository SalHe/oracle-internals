use evalexpr::{eval_boolean_with_context, Value, ContextWithMutableVariables};
use std::collections::HashMap;

/// Evaluate filter expression with given variables
pub fn matches(filter_expr: &str, variables: &HashMap<&str, i64>) -> anyhow::Result<bool> {
    let mut context= evalexpr::HashMapContext::<evalexpr::DefaultNumericTypes>::new();

    // Register all variables
    for (key, value) in variables {
        context.set_value(key.to_string(), Value::Int(*value))?;
    }

    // Evaluate expression
    eval_boolean_with_context(filter_expr, &context).map_err(|e| anyhow::anyhow!("{}", e))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_comparison() -> anyhow::Result<()> {
        let mut vars = HashMap::new();
        vars.insert("record_id", 5i64);
        assert!(matches("record_id == 5", &vars)?);
        assert!(!matches("record_id == 3", &vars)?);
        Ok(())
    }

    #[test]
    fn test_logical_or() -> anyhow::Result<()> {
        let mut vars = HashMap::new();
        vars.insert("record_id", 2i64);
        assert!(matches("record_id == 1 || record_id == 2 || record_id == 3", &vars)?);
        assert!(!matches("record_id == 5 || record_id == 6", &vars)?);
        Ok(())
    }

    #[test]
    fn test_complex_expression() -> anyhow::Result<()> {
        let mut vars = HashMap::new();
        vars.insert("record_id", 1i64);
        vars.insert("change_layer", 11i64);
        assert!(matches(
            "(record_id == 1 || record_id == 2 || record_id == 3) && change_layer == 11",
            &vars
        )?);
        assert!(matches("record_id == 1 && change_layer == 11", &vars)?);
        assert!(!matches("record_id == 5 && change_layer == 11", &vars)?);
        Ok(())
    }

    #[test]
    fn test_range() -> anyhow::Result<()> {
        let mut vars = HashMap::new();
        vars.insert("record_id", 5i64);
        assert!(matches("record_id >= 1 && record_id <= 10", &vars)?);
        assert!(!matches("record_id > 5 && record_id < 10", &vars)?);
        Ok(())
    }
}
