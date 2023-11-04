pub use chumsky::prelude::*;
use labelme_rs::indexmap::IndexMap;
use labelme_rs::serde_json;
pub use labelme_rs::{FlagSet, LabelMeData, Point};
use std::error;
use std::fmt;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
use thiserror::Error;

pub mod cli;

#[derive(Clone, Debug)]
pub enum Expr {
    Num(isize),
    Var(String),

    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Cmp(Box<Expr>, CmpOp, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum CmpOp {
    LE,
    LT,
    GE,
    GT,
    Eq,
    NotEq,
}

pub fn parser() -> impl Parser<char, Expr, Error = Simple<char>> {
    let ident = text::ident().padded();

    let expr = recursive(|expr| {
        let int = text::int(10)
            .map(|s: String| Expr::Num(s.parse().unwrap()))
            .padded();

        let atom = int
            .or(expr.delimited_by(just('('), just(')')))
            .or(ident.map(Expr::Var));

        let op = |c| just(c).padded();

        let unary = op('-')
            .repeated()
            .then(atom)
            .foldr(|_op, rhs| Expr::Neg(Box::new(rhs)));

        let product = unary
            .clone()
            .then(
                op('*')
                    .to(Expr::Mul as fn(_, _) -> _)
                    .then(unary)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        let sum = product
            .clone()
            .then(
                op('+')
                    .to(Expr::Add as fn(_, _) -> _)
                    .or(op('-').to(Expr::Sub as fn(_, _) -> _))
                    .then(product)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        let cmp_op = just("==".to_string())
            .to(CmpOp::Eq)
            .or(just("!=".to_string()).to(CmpOp::NotEq))
            .or(just("<=".to_string()).to(CmpOp::LE))
            .or(just("<".to_string()).to(CmpOp::LT))
            .or(just(">=".to_string()).to(CmpOp::GE))
            .or(just(">".to_string()).to(CmpOp::GT));

        sum.clone()
            .then(cmp_op.then(sum).repeated())
            .foldl(|a, (op, b)| Expr::Cmp(Box::new(a), op, Box::new(b)))
    });

    expr.then_ignore(end())
}

pub fn eval<'a>(expr: &'a Expr, vars: &Vec<(&'a String, isize)>) -> Result<isize, (isize, isize)> {
    match expr {
        Expr::Num(x) => Ok(*x),
        Expr::Neg(a) => Ok(-eval(a, vars)?),
        Expr::Add(a, b) => Ok(eval(a, vars)? + eval(b, vars)?),
        Expr::Sub(a, b) => Ok(eval(a, vars)? - eval(b, vars)?),
        Expr::Mul(a, b) => Ok(eval(a, vars)? * eval(b, vars)?),
        Expr::Cmp(a, op, b) => {
            let a = eval(a, vars)?;
            let b = eval(b, vars)?;
            let ret = match op {
                CmpOp::Eq => a == b,
                CmpOp::NotEq => a != b,
                CmpOp::LE => a <= b,
                CmpOp::LT => a < b,
                CmpOp::GE => a >= b,
                CmpOp::GT => a > b,
            };
            if ret {
                Ok(1)
            } else {
                Err((a, b))
            }
        }
        Expr::Var(name) => {
            if let Some((_, val)) = vars.iter().rev().find(|(var, _)| *var == name) {
                Ok(*val)
            } else {
                Ok(0)
            }
        }
    }
}

pub fn load_rules(filename: &Path) -> std::io::Result<Vec<String>> {
    let rules: Vec<String> = BufReader::new(File::open(filename)?)
        .lines()
        .map_while(Result::ok)
        .collect();
    Ok(rules)
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("parse error: {0}")]
    Error(String),
}

/// Parse rules
/// ```
/// let ast = lmrs::parse_rules(&vec!["a == b".into()]);
/// assert!(ast.is_ok());
/// let ast = lmrs::parse_rules(&vec!["a = b".into()]);
/// assert!(ast.is_err());
/// ```
pub fn parse_rules(rules: &[String]) -> Result<Vec<Expr>, ParseError> {
    let asts: Result<Vec<_>, _> = rules.iter().map(|r| parser().parse(r.clone())).collect();
    asts.map_err(|parse_errs| {
        let errs: Vec<_> = parse_errs
            .into_iter()
            .map(|e| format!("Parse error: {e}"))
            .collect();
        ParseError::Error(errs.join("\n"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    FileNotFound,
    InvalidJson(String),
    EvaluatedFalse(String, (isize, isize)),
    EvaluatedMultipleFalses(Vec<(String, (isize, isize))>),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::EvaluatedFalse(cond, (c1, c2)) => {
                write!(f, "Unsatisfied rule; \"{cond}\": {c1} vs. {c2}")
            }
            CheckError::EvaluatedMultipleFalses(errors) => {
                write!(f, "Unsatisfied rules;")?;
                let msg = errors
                    .iter()
                    .map(|(cond, (c1, c2))| format!(" \"{cond}\": {c1} vs. {c2}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                f.write_str(&msg)
            }
            _ => write!(f, "{self:?}"),
        }
    }
}

impl error::Error for CheckError {}

#[derive(PartialEq, Eq, Debug)]
pub enum CheckResult {
    Skipped,
    Passed,
}

pub fn check_json_file(
    rules: &[String],
    asts: &[Expr],
    json_filename: &Path,
    flags: &FlagSet,
    ignores: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_data: LabelMeData = serde_json::from_reader(BufReader::new(
        File::open(json_filename).or(Err(CheckError::FileNotFound))?,
    ))
    .map_err(|err| CheckError::InvalidJson(format!("{err}")))?;
    check_json(rules, asts, json_data, flags, ignores)
}

pub fn check_jsons(
    rules: &[String],
    asts: &[Expr],
    json_str: &str,
    flags: &FlagSet,
    ignores: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_data: LabelMeData =
        serde_json::from_str(json_str).map_err(|err| CheckError::InvalidJson(format!("{err}")))?;
    check_json(rules, asts, json_data, flags, ignores)
}

pub fn check_json(
    rules: &[String],
    asts: &[Expr],
    json_data: LabelMeData,
    flags: &FlagSet,
    ignores: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_flags: FlagSet = json_data
        .flags
        .into_iter()
        .filter_map(|(k, v)| if v { Some(k) } else { None })
        .collect();
    if (!flags.is_empty() && json_flags.intersection(flags).count() == 0)
        || json_flags.intersection(ignores).count() > 0
    {
        return Ok(CheckResult::Skipped);
    }
    let mut point_map: IndexMap<String, Vec<Point>> = IndexMap::new();
    for shape in json_data.shapes.into_iter() {
        let vec: &mut Vec<Point> = point_map.entry(shape.label).or_default();
        vec.push(shape.points[0]);
    }
    let vars: Vec<_> = point_map
        .iter()
        .map(|(k, v)| (k, v.len() as isize))
        .collect();

    let mut errors: Vec<_> = asts
        .iter()
        .zip(rules.iter())
        .filter_map(|(ast, rule)| {
            let result = eval(ast, &vars);
            match result {
                Ok(_) => None,
                Err(vals) => Some((rule.clone(), vals)),
            }
        })
        .collect();
    if errors.is_empty() {
        Ok(CheckResult::Passed)
    } else if errors.len() == 1 {
        let (rule, vals) = errors.pop().unwrap();
        Err(CheckError::EvaluatedFalse(rule, vals))
    } else {
        Err(CheckError::EvaluatedMultipleFalses(errors))
    }
}

/// Merge `right` object into `left` object
///
/// # Examples
///
/// Two JSON objects with no common key.
/// ```
/// let mut o1 = jzon::parse(r#"{"a": "b"}"#).unwrap();
/// let o2 = jzon::parse(r#"{"c": "d"}"#).unwrap();
/// lmrs::merge(&mut o1, o2);
/// assert_eq!(o1.to_string(), r#"{"a":"b","c":"d"}"#);
/// ```
///
/// Merging arrays.
/// ```
/// let mut o1 = jzon::parse(r#"{"a": [1]}"#).unwrap();
/// let o2 = jzon::parse(r#"{"a": [2]}"#).unwrap();
/// lmrs::merge(&mut o1, o2);
/// assert_eq!(o1.to_string(), r#"{"a":[1,2]}"#);
/// ```
///
/// Merging objects.
/// ```
/// let mut o1 = jzon::parse(r#"{"a": {"b":1}}"#).unwrap();
/// let o2 = jzon::parse(r#"{"a": {"c":1}}"#).unwrap();
/// lmrs::merge(&mut o1, o2);
/// assert_eq!(o1.to_string(), r#"{"a":{"b":1,"c":1}}"#);
/// ```
///
/// # Bad examples
///
/// Merging two objects with the duplicated keys
/// ```should_panic
/// let mut o1 = jzon::parse(r#"{"a": "b"}"#).unwrap();
/// let o2 = jzon::parse(r#"{"a": "d"}"#).unwrap();
/// lmrs::merge(&mut o1, o2);
/// ```
///
/// Merging two nested objects with the duplicated keys
/// ```should_panic
/// let mut o1 = jzon::parse(r#"{"a": {"b":1}}"#).unwrap();
/// let o2 = jzon::parse(r#"{"a": {"b":2}}"#).unwrap();
/// lmrs::merge(&mut o1, o2);
/// ```
pub fn merge(left: &mut jzon::JsonValue, right: jzon::JsonValue) {
    let left = if let jzon::JsonValue::Object(left) = left {
        left
    } else {
        panic!("Invalid json input");
    };
    let right = if let jzon::JsonValue::Object(right) = right {
        right
    } else {
        panic!("Invalid json input");
    };
    for (key, r_value) in right.into_iter() {
        if let Some(l_value) = left.get_mut(&key) {
            match l_value {
                jzon::JsonValue::Array(l) => {
                    if let jzon::JsonValue::Array(r) = r_value {
                        l.extend(r);
                    } else {
                        panic!(
                            "Invalid right-hand side type {:?} for left-hand side type (Array)",
                            r_value
                        );
                    };
                }
                jzon::JsonValue::Object(_) => {
                    if let jzon::JsonValue::Object(_) = r_value {
                        merge(l_value, r_value);
                    } else {
                        panic!("Invalid right-hand side type {:?} for left-hand side type (Object/Map)", r_value);
                    };
                }
                l => panic!(
                    "Trying to join to invalid type other than array or map: {:?} vs {:?}",
                    l, r_value
                ),
            }
        } else {
            left.insert(&key, r_value)
        }
    }
}

#[test]
fn test_check_json() {
    use std::path::PathBuf;
    let rule = "TL > 0".to_string();
    let rules = vec![rule];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );

    let rule = "X == 0".to_string();
    let rules = vec![rule];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Non-existent variable"
    );

    let rule = "TL == 0".to_string();
    let rules = vec![rule.clone()];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/img1.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, (1, 0)),
        "False rule"
    );
    let (rule1, rule2) = ("TL == 0".to_string(), "TR == 1".to_string());
    let rules = vec![rule1.clone(), rule2.clone()];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let errors = vec![(rule1, (1, 0)), (rule2, (0, 1))];
    filename.push("tests/img1.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedMultipleFalses(errors),
        "False rule"
    );

    let rule = "TL == TR".to_string();
    let rules = vec![rule];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/test.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap(),
        CheckResult::Passed,
        "Valid rule"
    );
    assert_eq!(
        check_json_file(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["f1".into()]),
            &FlagSet::new()
        )
        .unwrap(),
        CheckResult::Passed,
        "Test for a true flag"
    );
    assert_eq!(
        check_json_file(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["f2".into()]),
            &FlagSet::new()
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for a false flag"
    );
    assert_eq!(
        check_json_file(
            &rules,
            &asts,
            &filename,
            &FlagSet::new(),
            &FlagSet::from_iter(vec!["f1".into()])
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for ignoring flag"
    );
    assert_eq!(
        check_json_file(
            &rules,
            &asts,
            &filename,
            &FlagSet::from_iter(vec!["fx".into()]),
            &FlagSet::new()
        )
        .unwrap(),
        CheckResult::Skipped,
        "Test for a non-existent flag"
    );

    let rule = "TL == BL + 1".to_string();
    let rules = vec![rule.clone()];
    let asts = parse_rules(&rules).unwrap();
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.push("tests/test.json");
    assert_eq!(
        check_json_file(&rules, &asts, &filename, &FlagSet::new(), &FlagSet::new()).unwrap_err(),
        CheckError::EvaluatedFalse(rule, (1, 2)),
        "False rule"
    );
}
