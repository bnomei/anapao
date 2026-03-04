//! Deterministic expression parsing and evaluation runtime.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
/// Errors returned by expression parsing/evaluation.
pub enum ExprError {
    #[error("expression is empty")]
    Empty,
    #[error("invalid number literal `{literal}`")]
    InvalidNumber { literal: String },
    #[error("unexpected token `{token}` at byte {position}")]
    UnexpectedToken { token: String, position: usize },
    #[error("unknown variable `{name}`")]
    UnknownVariable { name: String },
    #[error("unknown function `{name}`")]
    UnknownFunction { name: String },
    #[error("function `{name}` arity mismatch: expected {expected}, got {actual}")]
    FunctionArity { name: String, expected: &'static str, actual: usize },
    #[error("division by zero")]
    DivisionByZero,
    #[error("non-finite expression result")]
    NonFiniteResult,
    #[error("expression graph cannot resolve dependencies for: {remaining:?}")]
    GraphUnresolved { remaining: Vec<String> },
}

#[derive(Debug, Clone, Default)]
/// Stateless runtime for evaluating arithmetic expressions.
pub struct ExprRuntime;

#[derive(Debug, Clone, PartialEq)]
/// Compiled expression AST for repeated deterministic evaluation.
pub(crate) struct CompiledExpr {
    ast: Expr,
}

impl ExprRuntime {
    /// Creates a new expression runtime.
    pub fn new() -> Self {
        Self
    }

    /// Evaluates one expression against a variable map.
    pub fn evaluate(
        &self,
        expression: &str,
        variables: &BTreeMap<String, f64>,
    ) -> Result<f64, ExprError> {
        let compiled = self.compile(expression)?;
        self.evaluate_compiled(&compiled, variables)
    }

    /// Parses an expression once for reuse across repeated evaluations.
    pub(crate) fn compile(&self, expression: &str) -> Result<CompiledExpr, ExprError> {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return Err(ExprError::Empty);
        }

        let ast = Parser::new(trimmed).parse()?;
        Ok(CompiledExpr { ast })
    }

    /// Evaluates a precompiled expression against a variable map.
    pub(crate) fn evaluate_compiled(
        &self,
        expression: &CompiledExpr,
        variables: &BTreeMap<String, f64>,
    ) -> Result<f64, ExprError> {
        eval_node(&expression.ast, variables)
    }

    /// Evaluates a precompiled expression using a variable resolver closure.
    pub(crate) fn evaluate_compiled_with_resolver<F>(
        &self,
        expression: &CompiledExpr,
        mut resolve: F,
    ) -> Result<f64, ExprError>
    where
        F: FnMut(&str) -> Option<f64>,
    {
        eval_node_with_resolver(&expression.ast, &mut resolve)
    }

    /// Evaluates a dependency graph of named expressions.
    pub fn evaluate_graph(
        &self,
        graph: &BTreeMap<String, String>,
        base_variables: &BTreeMap<String, f64>,
    ) -> Result<BTreeMap<String, f64>, ExprError> {
        let mut parsed = BTreeMap::<String, Expr>::new();
        let mut deps = BTreeMap::<String, BTreeSet<String>>::new();
        for (name, expression) in graph {
            let ast = self.compile(expression)?.ast;
            let mut refs = BTreeSet::new();
            collect_variable_refs(&ast, &mut refs);
            refs.remove(name);
            deps.insert(name.clone(), refs);
            parsed.insert(name.clone(), ast);
        }

        let mut values = base_variables.clone();
        let mut pending = parsed.keys().cloned().collect::<BTreeSet<_>>();

        while !pending.is_empty() {
            let mut progressed = false;
            let keys = pending.iter().cloned().collect::<Vec<_>>();
            for key in keys {
                let ready = deps
                    .get(&key)
                    .map(|required| required.iter().all(|dep| values.contains_key(dep)))
                    .unwrap_or(true);
                if !ready {
                    continue;
                }

                let ast = parsed.get(&key).expect("pending keys always parsed");
                let value = eval_node(ast, &values)?;
                values.insert(key.clone(), value);
                pending.remove(&key);
                progressed = true;
            }

            if !progressed {
                return Err(ExprError::GraphUnresolved {
                    remaining: pending.into_iter().collect::<Vec<_>>(),
                });
            }
        }

        Ok(values)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Number(f64),
    Variable(String),
    Unary { op: UnaryOp, value: Box<Expr> },
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    LParen,
    RParen,
    Comma,
    End,
}

impl Token {
    fn label(&self) -> String {
        match self {
            Token::Number(value) => value.to_string(),
            Token::Ident(name) => name.clone(),
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Star => "*".to_string(),
            Token::Slash => "/".to_string(),
            Token::Percent => "%".to_string(),
            Token::Less => "<".to_string(),
            Token::LessEqual => "<=".to_string(),
            Token::Greater => ">".to_string(),
            Token::GreaterEqual => ">=".to_string(),
            Token::EqualEqual => "==".to_string(),
            Token::BangEqual => "!=".to_string(),
            Token::LParen => "(".to_string(),
            Token::RParen => ")".to_string(),
            Token::Comma => ",".to_string(),
            Token::End => "<end>".to_string(),
        }
    }
}

struct Lexer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, bytes: input.as_bytes(), pos: 0 }
    }

    fn next_token(&mut self) -> Result<(Token, usize), ExprError> {
        self.skip_spaces();
        let position = self.pos;
        let Some(byte) = self.bytes.get(self.pos).copied() else {
            return Ok((Token::End, position));
        };

        let token = match byte {
            b'+' => {
                self.pos += 1;
                Token::Plus
            }
            b'-' => {
                self.pos += 1;
                Token::Minus
            }
            b'*' => {
                self.pos += 1;
                Token::Star
            }
            b'/' => {
                self.pos += 1;
                Token::Slash
            }
            b'%' => {
                self.pos += 1;
                Token::Percent
            }
            b'<' => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'=')) {
                    self.pos += 1;
                    Token::LessEqual
                } else {
                    Token::Less
                }
            }
            b'>' => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'=')) {
                    self.pos += 1;
                    Token::GreaterEqual
                } else {
                    Token::Greater
                }
            }
            b'=' => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'=')) {
                    self.pos += 1;
                    Token::EqualEqual
                } else {
                    return Err(ExprError::UnexpectedToken { token: "=".to_string(), position });
                }
            }
            b'!' => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'=')) {
                    self.pos += 1;
                    Token::BangEqual
                } else {
                    return Err(ExprError::UnexpectedToken { token: "!".to_string(), position });
                }
            }
            b'(' => {
                self.pos += 1;
                Token::LParen
            }
            b')' => {
                self.pos += 1;
                Token::RParen
            }
            b',' => {
                self.pos += 1;
                Token::Comma
            }
            b'0'..=b'9' | b'.' => self.lex_number()?,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident(),
            _ => {
                return Err(ExprError::UnexpectedToken {
                    token: (byte as char).to_string(),
                    position,
                })
            }
        };

        Ok((token, position))
    }

    fn skip_spaces(&mut self) {
        while let Some(byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn lex_ident(&mut self) -> Token {
        let start = self.pos;
        while let Some(byte) = self.bytes.get(self.pos) {
            if byte.is_ascii_alphanumeric() || *byte == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        Token::Ident(self.input[start..self.pos].to_string())
    }

    fn lex_number(&mut self) -> Result<Token, ExprError> {
        let start = self.pos;
        let mut seen_dot = false;
        let mut seen_exp = false;

        while let Some(byte) = self.bytes.get(self.pos) {
            match *byte {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !seen_dot && !seen_exp => {
                    seen_dot = true;
                    self.pos += 1;
                }
                b'e' | b'E' if !seen_exp => {
                    seen_exp = true;
                    self.pos += 1;
                    if let Some(sign) = self.bytes.get(self.pos) {
                        if *sign == b'+' || *sign == b'-' {
                            self.pos += 1;
                        }
                    }
                }
                _ => break,
            }
        }

        let literal = &self.input[start..self.pos];
        let value = literal
            .parse::<f64>()
            .map_err(|_| ExprError::InvalidNumber { literal: literal.to_string() })?;
        Ok(Token::Number(value))
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: (Token, usize),
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current =
            lexer.next_token().expect("lexer cannot fail before first token for static input");
        Self { lexer, current }
    }

    fn parse(mut self) -> Result<Expr, ExprError> {
        let expr = self.parse_expression()?;
        if !matches!(self.current.0, Token::End) {
            return Err(ExprError::UnexpectedToken {
                token: self.current.0.label(),
                position: self.current.1,
            });
        }
        Ok(expr)
    }

    fn parse_expression(&mut self) -> Result<Expr, ExprError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.current.0 {
                Token::Greater => BinaryOp::Greater,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                Token::Less => BinaryOp::Less,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::EqualEqual => BinaryOp::Equal,
                Token::BangEqual => BinaryOp::NotEqual,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_additive()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.current.0 {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_term()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current.0 {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ExprError> {
        match self.current.0 {
            Token::Plus => {
                self.bump()?;
                Ok(Expr::Unary { op: UnaryOp::Plus, value: Box::new(self.parse_unary()?) })
            }
            Token::Minus => {
                self.bump()?;
                Ok(Expr::Unary { op: UnaryOp::Minus, value: Box::new(self.parse_unary()?) })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ExprError> {
        match self.current.0.clone() {
            Token::Number(value) => {
                self.bump()?;
                Ok(Expr::Number(value))
            }
            Token::Ident(name) => {
                self.bump()?;
                if matches!(self.current.0, Token::LParen) {
                    self.bump()?;
                    let mut args = Vec::new();
                    if !matches!(self.current.0, Token::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current.0, Token::Comma) {
                                self.bump()?;
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            Token::LParen => {
                self.bump()?;
                let inner = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            _ => Err(ExprError::UnexpectedToken {
                token: self.current.0.label(),
                position: self.current.1,
            }),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ExprError> {
        if std::mem::discriminant(&self.current.0) == std::mem::discriminant(&expected) {
            self.bump()
        } else {
            Err(ExprError::UnexpectedToken {
                token: self.current.0.label(),
                position: self.current.1,
            })
        }
    }

    fn bump(&mut self) -> Result<(), ExprError> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }
}

fn eval_node(expr: &Expr, variables: &BTreeMap<String, f64>) -> Result<f64, ExprError> {
    let value = match expr {
        Expr::Number(value) => *value,
        Expr::Variable(name) => {
            *variables.get(name).ok_or_else(|| ExprError::UnknownVariable { name: name.clone() })?
        }
        Expr::Unary { op, value } => {
            let inner = eval_node(value, variables)?;
            match op {
                UnaryOp::Plus => inner,
                UnaryOp::Minus => -inner,
            }
        }
        Expr::Binary { op, left, right } => {
            let lhs = eval_node(left, variables)?;
            let rhs = eval_node(right, variables)?;
            match op {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Div => {
                    if rhs == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    lhs / rhs
                }
                BinaryOp::Mod => {
                    if rhs == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    lhs % rhs
                }
                BinaryOp::Greater => bool_to_f64(lhs > rhs),
                BinaryOp::GreaterEqual => bool_to_f64(lhs >= rhs),
                BinaryOp::Less => bool_to_f64(lhs < rhs),
                BinaryOp::LessEqual => bool_to_f64(lhs <= rhs),
                BinaryOp::Equal => bool_to_f64(lhs == rhs),
                BinaryOp::NotEqual => bool_to_f64(lhs != rhs),
            }
        }
        Expr::Call { name, args } => eval_call(name, args, variables)?,
    };

    if value.is_finite() {
        Ok(value)
    } else {
        Err(ExprError::NonFiniteResult)
    }
}

fn eval_node_with_resolver<F>(expr: &Expr, resolve: &mut F) -> Result<f64, ExprError>
where
    F: FnMut(&str) -> Option<f64>,
{
    let value = match expr {
        Expr::Number(value) => *value,
        Expr::Variable(name) => {
            resolve(name).ok_or_else(|| ExprError::UnknownVariable { name: name.clone() })?
        }
        Expr::Unary { op, value } => {
            let inner = eval_node_with_resolver(value, resolve)?;
            match op {
                UnaryOp::Plus => inner,
                UnaryOp::Minus => -inner,
            }
        }
        Expr::Binary { op, left, right } => {
            let lhs = eval_node_with_resolver(left, resolve)?;
            let rhs = eval_node_with_resolver(right, resolve)?;
            match op {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Div => {
                    if rhs == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    lhs / rhs
                }
                BinaryOp::Mod => {
                    if rhs == 0.0 {
                        return Err(ExprError::DivisionByZero);
                    }
                    lhs % rhs
                }
                BinaryOp::Greater => bool_to_f64(lhs > rhs),
                BinaryOp::GreaterEqual => bool_to_f64(lhs >= rhs),
                BinaryOp::Less => bool_to_f64(lhs < rhs),
                BinaryOp::LessEqual => bool_to_f64(lhs <= rhs),
                BinaryOp::Equal => bool_to_f64(lhs == rhs),
                BinaryOp::NotEqual => bool_to_f64(lhs != rhs),
            }
        }
        Expr::Call { name, args } => eval_call_with_resolver(name, args, resolve)?,
    };

    if value.is_finite() {
        Ok(value)
    } else {
        Err(ExprError::NonFiniteResult)
    }
}

fn eval_call(
    name: &str,
    args: &[Expr],
    variables: &BTreeMap<String, f64>,
) -> Result<f64, ExprError> {
    let evaluated =
        args.iter().map(|arg| eval_node(arg, variables)).collect::<Result<Vec<_>, _>>()?;

    match name {
        "log" => log_fn(&evaluated),
        "ln" => unary("ln", &evaluated, f64::ln),
        "exp" => unary("exp", &evaluated, f64::exp),
        "abs" => unary("abs", &evaluated, f64::abs),
        "floor" => unary("floor", &evaluated, f64::floor),
        "ceil" => unary("ceil", &evaluated, f64::ceil),
        "round" => unary("round", &evaluated, f64::round),
        "sqrt" => unary("sqrt", &evaluated, f64::sqrt),
        "mod" => modulo("mod", &evaluated),
        "min" => binary("min", &evaluated, f64::min),
        "max" => binary("max", &evaluated, f64::max),
        "pow" => binary("pow", &evaluated, f64::powf),
        "clamp" => ternary("clamp", &evaluated, |value, min, max| value.clamp(min, max)),
        _ => Err(ExprError::UnknownFunction { name: name.to_string() }),
    }
}

fn eval_call_with_resolver<F>(name: &str, args: &[Expr], resolve: &mut F) -> Result<f64, ExprError>
where
    F: FnMut(&str) -> Option<f64>,
{
    let evaluated = args
        .iter()
        .map(|arg| eval_node_with_resolver(arg, resolve))
        .collect::<Result<Vec<_>, _>>()?;

    match name {
        "log" => log_fn(&evaluated),
        "ln" => unary("ln", &evaluated, f64::ln),
        "exp" => unary("exp", &evaluated, f64::exp),
        "abs" => unary("abs", &evaluated, f64::abs),
        "floor" => unary("floor", &evaluated, f64::floor),
        "ceil" => unary("ceil", &evaluated, f64::ceil),
        "round" => unary("round", &evaluated, f64::round),
        "sqrt" => unary("sqrt", &evaluated, f64::sqrt),
        "mod" => modulo("mod", &evaluated),
        "min" => binary("min", &evaluated, f64::min),
        "max" => binary("max", &evaluated, f64::max),
        "pow" => binary("pow", &evaluated, f64::powf),
        "clamp" => ternary("clamp", &evaluated, |value, min, max| value.clamp(min, max)),
        _ => Err(ExprError::UnknownFunction { name: name.to_string() }),
    }
}

fn unary(name: &str, args: &[f64], f: fn(f64) -> f64) -> Result<f64, ExprError> {
    if args.len() != 1 {
        return Err(ExprError::FunctionArity {
            name: name.to_string(),
            expected: "1 argument",
            actual: args.len(),
        });
    }
    Ok(f(args[0]))
}

fn binary(name: &str, args: &[f64], f: fn(f64, f64) -> f64) -> Result<f64, ExprError> {
    if args.len() != 2 {
        return Err(ExprError::FunctionArity {
            name: name.to_string(),
            expected: "2 arguments",
            actual: args.len(),
        });
    }
    Ok(f(args[0], args[1]))
}

fn ternary(name: &str, args: &[f64], f: fn(f64, f64, f64) -> f64) -> Result<f64, ExprError> {
    if args.len() != 3 {
        return Err(ExprError::FunctionArity {
            name: name.to_string(),
            expected: "3 arguments",
            actual: args.len(),
        });
    }
    Ok(f(args[0], args[1], args[2]))
}

fn log_fn(args: &[f64]) -> Result<f64, ExprError> {
    match args {
        [value] => Ok(value.log10()),
        [value, base] => Ok(value.log(*base)),
        _ => Err(ExprError::FunctionArity {
            name: "log".to_string(),
            expected: "1 or 2 arguments",
            actual: args.len(),
        }),
    }
}

fn modulo(name: &str, args: &[f64]) -> Result<f64, ExprError> {
    if args.len() != 2 {
        return Err(ExprError::FunctionArity {
            name: name.to_string(),
            expected: "2 arguments",
            actual: args.len(),
        });
    }
    if args[1] == 0.0 {
        return Err(ExprError::DivisionByZero);
    }
    Ok(args[0] % args[1])
}

fn bool_to_f64(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn collect_variable_refs(expr: &Expr, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Number(_) => {}
        Expr::Variable(name) => {
            out.insert(name.clone());
        }
        Expr::Unary { value, .. } => collect_variable_refs(value, out),
        Expr::Binary { left, right, .. } => {
            collect_variable_refs(left, out);
            collect_variable_refs(right, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_variable_refs(arg, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{ExprError, ExprRuntime};

    #[test]
    fn evaluate_supports_whitelisted_math_functions_deterministically() {
        let runtime = ExprRuntime::new();
        let mut vars = BTreeMap::new();
        vars.insert("x".to_string(), 4.0);
        vars.insert("y".to_string(), 3.0);

        let value = runtime
            .evaluate("max(x, y) + pow(2, 3) + floor(2.8) + clamp(-1, 0, 2)", &vars)
            .expect("expression should evaluate");

        assert_eq!(value, 4.0 + 8.0 + 2.0 + 0.0);
    }

    #[test]
    fn evaluate_supports_extended_math_functions_deterministically() {
        let runtime = ExprRuntime::new();
        let value = runtime
            .evaluate(
                "log(100) + log(8, 2) + ln(exp(1)) + exp(0) + mod(10, 3) + (10 % 4)",
                &BTreeMap::new(),
            )
            .expect("expression should evaluate");

        assert!((value - 10.0).abs() < 1e-12, "expected 10.0, got {value}");
    }

    #[test]
    fn evaluate_supports_comparison_operators_with_numeric_boolean_results() {
        let runtime = ExprRuntime::new();

        assert_eq!(runtime.evaluate("3 > 2", &BTreeMap::new()).expect("comparison"), 1.0);
        assert_eq!(runtime.evaluate("3 >= 3", &BTreeMap::new()).expect("comparison"), 1.0);
        assert_eq!(runtime.evaluate("2 < 1", &BTreeMap::new()).expect("comparison"), 0.0);
        assert_eq!(runtime.evaluate("2 <= 1", &BTreeMap::new()).expect("comparison"), 0.0);
        assert_eq!(runtime.evaluate("2 == 2", &BTreeMap::new()).expect("comparison"), 1.0);
        assert_eq!(runtime.evaluate("2 != 2", &BTreeMap::new()).expect("comparison"), 0.0);
    }

    #[test]
    fn evaluate_comparison_has_lower_precedence_than_arithmetic() {
        let runtime = ExprRuntime::new();
        let value = runtime
            .evaluate("1 + 2 > 2 * 1", &BTreeMap::new())
            .expect("expression should evaluate");
        assert_eq!(value, 1.0);
    }

    #[test]
    fn evaluate_is_case_sensitive_for_variables() {
        let runtime = ExprRuntime::new();
        let mut vars = BTreeMap::new();
        vars.insert("value".to_string(), 5.0);

        let err = runtime.evaluate("Value + 1", &vars).expect_err("case mismatch should fail");
        assert!(matches!(err, ExprError::UnknownVariable { name } if name == "Value"));
    }

    #[test]
    fn evaluate_rejects_unknown_function() {
        let runtime = ExprRuntime::new();
        let vars = BTreeMap::new();
        let err =
            runtime.evaluate("sin(1)", &vars).expect_err("non-whitelisted function should fail");
        assert!(matches!(err, ExprError::UnknownFunction { name } if name == "sin"));
    }

    #[test]
    fn evaluate_rejects_empty_expression() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("   ", &BTreeMap::new()).expect_err("empty input should fail");
        assert!(matches!(err, ExprError::Empty));
    }

    #[test]
    fn evaluate_rejects_invalid_number_literal() {
        let runtime = ExprRuntime::new();
        let err =
            runtime.evaluate("0 + 1e+", &BTreeMap::new()).expect_err("invalid number must fail");
        assert!(matches!(err, ExprError::InvalidNumber { literal } if literal == "1e+"));
    }

    #[test]
    fn evaluate_reports_unexpected_token_with_position() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("1 + )", &BTreeMap::new()).expect_err("parse must fail");
        assert!(
            matches!(err, ExprError::UnexpectedToken { token, position } if token == ")" && position == 4)
        );
    }

    #[test]
    fn evaluate_reports_single_equals_token_as_unexpected() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("1 = 1", &BTreeMap::new()).expect_err("parse must fail");
        assert!(
            matches!(err, ExprError::UnexpectedToken { token, position } if token == "=" && position == 2)
        );
    }

    #[test]
    fn evaluate_reports_division_by_zero() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("10 / (3 - 3)", &BTreeMap::new()).expect_err("must reject /0");
        assert!(matches!(err, ExprError::DivisionByZero));
    }

    #[test]
    fn evaluate_reports_modulo_by_zero() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("10 % 0", &BTreeMap::new()).expect_err("must reject %0");
        assert!(matches!(err, ExprError::DivisionByZero));

        let err = runtime.evaluate("mod(10, 0)", &BTreeMap::new()).expect_err("must reject mod/0");
        assert!(matches!(err, ExprError::DivisionByZero));
    }

    #[test]
    fn evaluate_reports_non_finite_result() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("sqrt(-1)", &BTreeMap::new()).expect_err("nan must fail");
        assert!(matches!(err, ExprError::NonFiniteResult));
    }

    #[test]
    fn evaluate_reports_function_arity_mismatch() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("max(1)", &BTreeMap::new()).expect_err("arity must fail");
        assert!(matches!(
            err,
            ExprError::FunctionArity { name, expected, actual }
            if name == "max" && expected == "2 arguments" && actual == 1
        ));
    }

    #[test]
    fn evaluate_reports_new_function_arity_mismatch() {
        let runtime = ExprRuntime::new();
        let err = runtime.evaluate("mod(1)", &BTreeMap::new()).expect_err("arity must fail");
        assert!(matches!(
            err,
            ExprError::FunctionArity { name, expected, actual }
            if name == "mod" && expected == "2 arguments" && actual == 1
        ));
    }

    #[test]
    fn evaluate_honors_precedence_and_parentheses() {
        let runtime = ExprRuntime::new();
        let value = runtime
            .evaluate("1 + 2 * (3 + 4) - 5", &BTreeMap::new())
            .expect("expression should evaluate");
        assert_eq!(value, 10.0);
    }

    #[test]
    fn evaluate_graph_resolves_dependencies_in_stable_order() {
        let runtime = ExprRuntime::new();
        let mut graph = BTreeMap::new();
        graph.insert("b".to_string(), "a + 2".to_string());
        graph.insert("a".to_string(), "base + 1".to_string());
        graph.insert("c".to_string(), "max(a, b)".to_string());

        let mut base = BTreeMap::new();
        base.insert("base".to_string(), 3.0);

        let result = runtime.evaluate_graph(&graph, &base).expect("graph should resolve");
        assert_eq!(result.get("a"), Some(&4.0));
        assert_eq!(result.get("b"), Some(&6.0));
        assert_eq!(result.get("c"), Some(&6.0));
    }

    #[test]
    fn evaluate_graph_reports_unresolved_cycles() {
        let runtime = ExprRuntime::new();
        let mut graph = BTreeMap::new();
        graph.insert("a".to_string(), "b + 1".to_string());
        graph.insert("b".to_string(), "a + 1".to_string());

        let err =
            runtime.evaluate_graph(&graph, &BTreeMap::new()).expect_err("cyclic graph should fail");
        assert!(
            matches!(err, ExprError::GraphUnresolved { remaining } if remaining == vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn evaluate_graph_allows_self_reference_from_base_variables() {
        let runtime = ExprRuntime::new();
        let graph = BTreeMap::from([("a".to_string(), "a + 2".to_string())]);
        let base = BTreeMap::from([("a".to_string(), 5.0)]);

        let result = runtime.evaluate_graph(&graph, &base).expect("self-reference should resolve");
        assert_eq!(result.get("a"), Some(&7.0));
    }

    #[test]
    fn evaluate_graph_reports_unresolved_external_dependency() {
        let runtime = ExprRuntime::new();
        let graph = BTreeMap::from([("a".to_string(), "missing + 1".to_string())]);

        let err =
            runtime.evaluate_graph(&graph, &BTreeMap::new()).expect_err("missing dep must fail");
        assert!(
            matches!(err, ExprError::GraphUnresolved { remaining } if remaining == vec!["a".to_string()])
        );
    }

    #[test]
    fn compile_and_evaluate_compiled_are_reusable() {
        let runtime = ExprRuntime::new();
        let compiled = runtime.compile("x * 2 + 1").expect("compile expression");

        let vars_a = BTreeMap::from([("x".to_string(), 3.0)]);
        let vars_b = BTreeMap::from([("x".to_string(), 7.0)]);

        assert_eq!(runtime.evaluate_compiled(&compiled, &vars_a).expect("evaluate vars a"), 7.0);
        assert_eq!(runtime.evaluate_compiled(&compiled, &vars_b).expect("evaluate vars b"), 15.0);
    }

    #[test]
    fn evaluate_compiled_with_resolver_supports_lookup() {
        let runtime = ExprRuntime::new();
        let compiled = runtime.compile("max(step + 1, total)").expect("compile expression");
        let value = runtime
            .evaluate_compiled_with_resolver(&compiled, |name| match name {
                "step" => Some(2.0),
                "total" => Some(4.0),
                _ => None,
            })
            .expect("evaluate with resolver");

        assert_eq!(value, 4.0);
    }
}
