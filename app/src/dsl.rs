pub use chumsky::prelude::*;
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

        sum
            .clone()
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
        .filter_map(|l| l.ok())
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
pub fn parse_rules(rules: &[String]) -> Result<Vec<Expr>, Box<dyn std::error::Error>> {
    let asts: Result<Vec<_>, _> = rules.iter().map(|r| parser().parse(r.clone())).collect();
    asts.map_err(|parse_errs| {
        let errs: Vec<_> = parse_errs
            .into_iter()
            .map(|e| format!("Parse error: {}", e))
            .collect();
        errs.join("\n").into()
    })
}
