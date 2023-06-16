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
use regex::Regex;
#[macro_use]
extern crate lazy_static;

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

pub fn load_rules(filename: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let rules: Vec<String> = BufReader::new(File::open(filename)?)
        .lines()
        .map_while(Result::ok)
        .collect();
    Ok(rules)
}

/// Parse rules
/// ```
/// let ast = dsl::parse_rules(&vec!["a == b".into()]);
/// assert!(ast.is_ok());
/// let ast = dsl::parse_rules(&vec!["a = b".into()]);
/// assert!(ast.is_err());
/// ```
pub fn parse_rules(rules: &[String]) -> Result<Vec<Expr>, String> {
    let asts: Result<Vec<_>, _> = rules.iter().map(|r| parser().parse(r.clone())).collect();
    asts.map_err(|parse_errs| {
        let errs: Vec<_> = parse_errs
            .into_iter()
            .map(|e| format!("Parse error: {}", e))
            .collect();
        errs.join("\n")
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
                write!(f, "Unsatisfied rule; \"{}\": {} vs. {}", cond, c1, c2)
            }
            CheckError::EvaluatedMultipleFalses(errors) => {
                write!(f, "Unsatisfied rules;")?;
                let msg = errors
                    .iter()
                    .map(|(cond, (c1, c2))| format!(" \"{}\": {} vs. {}", cond, c1, c2))
                    .collect::<Vec<_>>()
                    .join(", ");
                f.write_str(&msg)
            }
            _ => write!(f, "{:?}", self),
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
    .map_err(|err| CheckError::InvalidJson(format!("{}", err)))?;
    check_json(rules, asts, json_data, flags, ignores)
}

pub fn check_jsons(
    rules: &[String],
    asts: &[Expr],
    json_str: &str,
    flags: &FlagSet,
    ignores: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_data: LabelMeData = serde_json::from_str(json_str)
        .map_err(|err| CheckError::InvalidJson(format!("{}", err)))?;
    check_json(rules, asts, json_data, flags, ignores)
}

pub fn check_json(
    rules: &[String],
    asts: &[Expr],
    json_data: LabelMeData,
    flags: &FlagSet,
    ignores: &FlagSet,
) -> Result<CheckResult, CheckError> {
    let json_flags =
        FlagSet::from_iter(json_data.flags.into_iter().filter_map(
            |(k, v)| {
                if v {
                    Some(k)
                } else {
                    None
                }
            },
        ));
    if (!flags.is_empty() && json_flags.intersection(flags).count() == 0)
        || json_flags.intersection(ignores).count() > 0
    {
        return Ok(CheckResult::Skipped);
    }
    let mut point_map: IndexMap<String, Vec<Point>> = IndexMap::new();
    for shape in json_data.shapes.into_iter() {
        let vec: &mut Vec<Point> = point_map.entry(shape.label).or_insert_with(Vec::new);
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

#[derive(Debug, PartialEq)]
pub enum ResizeParam {
    Percentage(f64),
    Size(u32, u32),
}

lazy_static! {
    static ref RE_PERCENT: Regex = Regex::new(r"^(\d+)%$").unwrap();
    static ref RE_SIZE: Regex = Regex::new(r"^(\d+)x(\d+)$").unwrap();
}

impl TryFrom<&str> for ResizeParam {
    type Error = Box<dyn std::error::Error>;

    /// Parse resize parameter
    /// ```
    /// assert_eq!(dsl::ResizeParam::try_from("33%").unwrap(), dsl::ResizeParam::Percentage(0.33));
    /// assert_eq!(dsl::ResizeParam::try_from("300x400").unwrap(), dsl::ResizeParam::Size(300, 400));
    /// ```
    fn try_from(param: &str) -> Result<Self, Self::Error> {
        if let Some(cap) = RE_PERCENT.captures(param) {
            let p: f64 = cap.get(1).unwrap().as_str().parse::<u8>()? as f64 / 100.0;
            return Ok(ResizeParam::Percentage(p));
        } else {
            if let Some(cap) = RE_SIZE.captures(param) {
                let w: u32 = cap.get(1).unwrap().as_str().parse()?;
                let h: u32 = cap.get(2).unwrap().as_str().parse()?;
                return Ok(ResizeParam::Size(w, h));
            } else {
                return Err(format!("{} is invalid resize argument", param).into());
            }
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
