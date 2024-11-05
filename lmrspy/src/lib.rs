use pyo3::prelude::*;

#[pyclass]
struct Validator {
    rules: Vec<String>,
    asts: Vec<lmrs::Expr>,
    flags: lmrs::FlagSet,
    ignores: lmrs::FlagSet,
}

fn concat<T, S: std::fmt::Display>(iterator: T, sep: &str) -> String
where
    T: Iterator<Item = S>,
{
    iterator
        .map(|rule| format!("'{}'", rule))
        .collect::<Vec<_>>()
        .join(sep)
}

#[pymethods]
impl Validator {
    #[new]
    fn new(rules: Vec<String>, flag_set: Vec<String>, ignore_set: Vec<String>) -> PyResult<Self> {
        let flags = lmrs::FlagSet::from_iter(flag_set);
        let ignores = lmrs::FlagSet::from_iter(ignore_set);
        match lmrs::parse_rules(&rules) {
            Ok(asts) => Ok(Self {
                rules,
                asts,
                flags,
                ignores,
            }),
            Err(err) => Err(pyo3::exceptions::PyValueError::new_err(format!("{}", err))),
        }
    }
    fn __repr__(&self) -> PyResult<String> {
        let rules = concat(self.rules.iter(), ", ");
        let flags = concat(self.flags.iter(), ", ");
        let ignores = concat(self.ignores.iter(), ", ");
        Ok(format!(
            "Validator([{}], [{}], [{}])",
            rules, flags, ignores
        ))
    }

    fn validate_jsons(&self, json_str: &str) -> PyResult<bool> {
        let check_result = lmrs::check_jsons(
            &self.rules,
            &self.asts,
            json_str,
            &self.flags,
            &self.ignores,
        );
        let result = match check_result {
            Ok(result) => Ok(result == lmrs::CheckResult::Passed),
            Err(err) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "NG : {}",
                err
            ))),
        }?;
        Ok(result)
    }

    fn validate_json(&self, filename: &str) -> PyResult<bool> {
        let filename = std::path::PathBuf::from(filename);
        let check_result = lmrs::check_json_file(
            &self.rules,
            &self.asts,
            &filename,
            &self.flags,
            &self.ignores,
        );
        let result = match check_result {
            Ok(result) => Ok(result == lmrs::CheckResult::Passed),
            Err(err) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "NG : {}",
                err
            ))),
        }?;
        Ok(result)
    }
}

#[pymodule]
fn lmrspy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Validator>()?;
    Ok(())
}
