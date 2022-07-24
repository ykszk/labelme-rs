pub use chumsky::prelude::*;

#[derive(Clone, Debug)]
pub enum Expr {
    Num(isize),
    Var(String),

    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Cmp(Box<Expr>, CmpOp, Box<Expr>),
    Cond(Box<Expr>, CondOp, Box<Expr>),
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

#[derive(Clone, Debug)]
pub enum CondOp {
    And,
    Or,
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

        let compare = sum
            .clone()
            .then(cmp_op.then(sum).repeated())
            .foldl(|a, (op, b)| Expr::Cmp(Box::new(a), op, Box::new(b)));

        let cond_op = just("&&".to_string())
            .to(CondOp::And)
            .or(just("||".to_string()).to(CondOp::Or));

        let cond = compare
            .clone()
            .then(cond_op.then(compare).repeated())
            .foldl(|a, (op, b)| Expr::Cond(Box::new(a), op, Box::new(b)));

        cond
    });

    expr.then_ignore(end())
}

pub fn eval<'a>(expr: &'a Expr, vars: &Vec<(&'a String, isize)>) -> Result<isize, String> {
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
                Err(format!("{} and {}", a, b))
            }
        }
        Expr::Cond(a, op, b) => {
            let a = eval(a, vars)? != 0;
            let b = eval(b, vars)? != 0;
            let ret = match op {
                CondOp::And => a && b,
                CondOp::Or => a || b,
            };
            Ok(if ret { 1 } else { 0 })
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
