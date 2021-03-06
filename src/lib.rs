extern crate num_bigint;
extern crate num_rational;
extern crate num_traits;

#[macro_use]
pub mod ast;
pub mod checker;
pub mod parser;
mod utils;

use checker::CheckerError;
use parser::error::ParserError;
use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
};

#[derive(Debug)]
pub enum Error {
    Parser(ParserError),
    Checker(CheckerError),
}

impl From<ParserError> for Error {
    fn from(e: ParserError) -> Self {
        Self::Parser(e)
    }
}

impl From<CheckerError> for Error {
    fn from(e: CheckerError) -> Self {
        Self::Checker(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Parser(e.into())
    }
}

pub fn check<P: AsRef<Path>>(
    problem_path: P,
    proof_path: P,
    skip_unknown_rules: bool,
    allow_test_rule: bool,
) -> Result<checker::Correctness, Error> {
    let (proof, pool) = parser::parse_problem_proof(
        BufReader::new(File::open(problem_path).unwrap()),
        BufReader::new(File::open(proof_path).unwrap()),
    )?;

    checker::ProofChecker::new(pool, skip_unknown_rules, allow_test_rule)
        .check(&proof)
        .map_err(Error::Checker)
}
