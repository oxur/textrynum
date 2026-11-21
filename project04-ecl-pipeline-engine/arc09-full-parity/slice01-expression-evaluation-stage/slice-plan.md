# 2. Milestone 10.1: Expression Evaluation Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 74-331. Original bytes are recoverable from import commit 3a39aea. -->

## 2. Milestone 10.1: Expression Evaluation Stage

### 2.1 Scope

A stage that evaluates expressions on record fields — arithmetic, conditionals, string operations. This replaces the need for custom per-customer Rust code for computed fields.

### 2.2 Why This Is Needed

| Use Case | Expression |
|----------|-----------|
| Acme Inc. digital price correction | `amount / 100` |
| BOPIS detection | `if bopisFlag == "Y" then "web" else order_type` |
| Chime reward calculation | `min(private_label * 0.05 + non_private * 0.03, 5.0)` |
| Cash back calculation | `if payment_type == "Debit Card/Cashback" and amount < 0 then abs(amount) else 0` |
| Net tender calculation | `total_amount - cash_back_amount` |

### 2.3 Expression Language Design

Use a simple, safe expression language. **Not** a general-purpose scripting language — no loops, no I/O, no function definitions. Just field references, literals, operators, and built-in functions.

**Syntax (inspired by jq/jsonpath with SQL-like conditionals):**

```
# Field reference
.field_name

# Literals
42, 3.14, "hello", true, null

# Arithmetic
.amount / 100
.price * .quantity
.total - .discount

# Comparison
.field == "value"
.amount > 0
.count >= 10

# Logical
.a && .b
.a || .b
!.flag

# Conditionals
if .bopisFlag == "Y" then "web" else .order_type

# Built-in functions
min(.reward, 5.0)
max(.a, .b)
abs(.amount)
round(.value, 2)
len(.name)
lower(.text)
upper(.text)
trim(.text)
concat(.first, " ", .last)
coalesce(.preferred, .fallback, "default")
```

### 2.4 Implementation

#### File: `crates/ecl-stages/src/expression.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionConfig {
    /// List of expressions to evaluate.
    pub expressions: Vec<ExpressionOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionOp {
    /// Output field name for the result.
    pub output: String,
    /// Expression string to evaluate.
    pub expr: String,
}
```

**Expression evaluator:**

Rather than building a custom parser, use the `evalexpr` crate which provides a safe expression evaluator with no I/O or side effects:

```toml
# In ecl-stages/Cargo.toml:
evalexpr = "14"
```

`evalexpr` supports: arithmetic, comparisons, logical operators, string operations, and custom functions. It operates on a `HashMapContext` of variable bindings.

```rust
use evalexpr::*;

#[derive(Debug)]
pub struct ExpressionStage {
    config: ExpressionConfig,
    /// Pre-compiled expressions.
    compiled: Vec<(String, Node)>,  // (output_field, compiled_expr)
}

impl ExpressionStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: ExpressionConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let compiled = config.expressions.iter()
            .map(|op| {
                let node = build_operator_tree(&op.expr)
                    .map_err(|e| StageError::Permanent {
                        stage: "expression".to_string(),
                        item_id: String::new(),
                        message: format!("invalid expression '{}': {e}", op.expr),
                    })?;
                Ok((op.output.clone(), node))
            })
            .collect::<Result<Vec<_>, StageError>>()?;

        Ok(Self { config, compiled })
    }
}

#[async_trait]
impl Stage for ExpressionStage {
    fn name(&self) -> &str { "expression" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        // Build evalexpr context from record fields.
        let mut context = HashMapContext::new();
        for (key, value) in record.iter() {
            match value {
                Value::String(s) => { context.set_value(key.clone(), evalexpr::Value::String(s.clone())).ok(); }
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        context.set_value(key.clone(), evalexpr::Value::Float(f)).ok();
                    } else if let Some(i) = n.as_i64() {
                        context.set_value(key.clone(), evalexpr::Value::Int(i)).ok();
                    }
                }
                Value::Bool(b) => { context.set_value(key.clone(), evalexpr::Value::Boolean(*b)).ok(); }
                Value::Null => { context.set_value(key.clone(), evalexpr::Value::Empty).ok(); }
                _ => {} // Skip arrays/objects
            }
        }

        // Register custom functions.
        context.set_function("coalesce".to_string(), Function::new(|args| {
            for arg in args.as_tuple()? {
                if arg != &evalexpr::Value::Empty {
                    return Ok(arg.clone());
                }
            }
            Ok(evalexpr::Value::Empty)
        })).ok();

        // Evaluate each expression.
        for (output, compiled_expr) in &self.compiled {
            let result = compiled_expr.eval_with_context(&context)
                .unwrap_or(evalexpr::Value::Empty);

            let json_value = evalexpr_to_json(&result);
            record.insert(output.clone(), json_value);

            // Also update context so later expressions can reference earlier results.
            context.set_value(output.clone(), result).ok();
        }

        Ok(vec![item])
    }
}

fn evalexpr_to_json(val: &evalexpr::Value) -> serde_json::Value {
    match val {
        evalexpr::Value::String(s) => serde_json::Value::String(s.clone()),
        evalexpr::Value::Float(f) => serde_json::json!(f),
        evalexpr::Value::Int(i) => serde_json::json!(i),
        evalexpr::Value::Boolean(b) => serde_json::Value::Bool(*b),
        evalexpr::Value::Empty => serde_json::Value::Null,
        evalexpr::Value::Tuple(t) => {
            serde_json::Value::Array(t.iter().map(evalexpr_to_json).collect())
        }
    }
}
```

**TOML spec example (Acme Inc. digital price correction):**

```toml
[[stages]]
name = "price_correct_digital"
handler = "expression"
input_streams = ["digital-transactions"]
output_stream = "corrected-transactions"
[stages.price_correct_digital.params]
expressions = [
  { output = "merchant_total_amount", expr = "merchant_total_amount / 100" },
  { output = "shipping_amount", expr = "shipping_amount / 100" },
  { output = "tax_amount", expr = "tax_amount / 100" },
  { output = "subtotal_amount", expr = "subtotal_amount / 100" },
  { output = "charged_amount", expr = "charged_amount / 100" },
]
```

**TOML spec example (BOPIS detection):**

```toml
[[stages]]
name = "bopis_detect"
handler = "expression"
[stages.bopis_detect.params]
expressions = [
  { output = "order_type", expr = "if(bopisFlag == \"Y\", \"web\", order_type)" },
  { output = "fulfillment_type", expr = "if(bopisFlag == \"Y\", \"in_store\", fulfillment_type)" },
]
```

**TOML spec example (Chime rewards):**

```toml
[[stages]]
name = "calc_rewards"
handler = "expression"
[stages.calc_rewards.params]
expressions = [
  { output = "private_label_reward", expr = "private_label_amt * 0.05" },
  { output = "non_private_reward", expr = "non_private_label_amt * 0.03" },
  { output = "raw_reward", expr = "private_label_reward + non_private_reward" },
  { output = "total_reward_amount", expr = "min(raw_reward, 5.0)" },
]
```

### 2.5 Tests

1. `test_expr_arithmetic_basic` — `a + b`
2. `test_expr_division` — `amount / 100`
3. `test_expr_conditional` — `if(flag == "Y", "web", "in_store")`
4. `test_expr_min_max` — `min(a, 5.0)`
5. `test_expr_abs` — `abs(negative_amount)`
6. `test_expr_string_functions` — `lower(name)`, `upper(name)`, `trim(name)`
7. `test_expr_coalesce` — first non-null value
8. `test_expr_chained_references` — later expression references earlier result
9. `test_expr_missing_field_evaluates_empty`
10. `test_expr_acme_inc_price_correction` — full digital price fix
11. `test_expr_chime_reward_calculation`
12. `test_expr_compile_error` — invalid expression → Permanent error
13. `test_expr_from_params_invalid`

---

