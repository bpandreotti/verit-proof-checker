mod tests;

use crate::parser::ast::*;

pub type Rule = fn(&[ByRefRc<Term>], Vec<&ProofCommand>, &[ProofArg]) -> Option<()>;

pub struct ProofChecker {
    proof: Proof,
}

impl ProofChecker {
    pub fn new(proof: Proof) -> Self {
        ProofChecker { proof }
    }

    pub fn check(self) -> bool {
        for step in &self.proof.0 {
            if let ProofCommand::Step {
                clause,
                rule,
                premises,
                args,
            } = step
            {
                let rule = Self::get_rule(rule).unwrap_or_else(|| panic!("unknown rule: {}", rule));
                let premises = premises.iter().map(|&i| &self.proof.0[i]).collect();
                if rule(&clause, premises, &args).is_none() {
                    return false;
                }
            }
        }
        true
    }

    pub fn get_rule(rule_name: &str) -> Option<Rule> {
        Some(match rule_name {
            "not_not" => rules::not_not,
            "equiv_pos1" => rules::equiv_pos1,
            "equiv_pos2" => rules::equiv_pos2,
            "eq_reflexive" => rules::eq_reflexive,
            "eq_transitive" => rules::eq_transitive,
            "eq_congruent" | "eq_congruent_pred" => rules::eq_congruent,
            "distinct_elim" => rules::distinct_elim,
            "th_resolution" | "resolution" => rules::resolution,
            "and" => rules::and,
            "or" => rules::or,
            "ite1" => rules::ite1,
            "ite2" => rules::ite2,
            "ite_intro" => rules::ite_intro,
            "contraction" => rules::contraction,
            _ => return None,
        })
    }
}

/// A macro to help deconstruct operation terms. Since a term holds references to other terms in
/// `Vec`s and `Rc`s, pattern matching a complex term can be difficult and verbose. This macro
/// helps with that.
macro_rules! match_op {
    ($bind:ident = $var:expr) => {
        Some($var)
    };
    (($op:tt $($args:tt)+) = $var:expr) => {{
        let _: &Term = $var;
        if let Term::Op(match_op!(@GET_VARIANT $op), args) = $var {
            match_op!(@ARGS ($($args)+) = args.as_slice())
        } else {
            None
        }
    }};
    (@ARGS ($arg:tt) = $var:expr) => {
        match_op!(@ARGS_IDENT (arg1: $arg) = $var)
    };
    (@ARGS ($arg1:tt $arg2:tt) = $var:expr) => {
        match_op!(@ARGS_IDENT (arg1: $arg1, arg2: $arg2) = $var)
    };
    (@ARGS ($arg1:tt $arg2:tt $arg3:tt) = $var:expr) => {
        match_op!(@ARGS_IDENT (arg1: $arg1, arg2: $arg2, arg3: $arg3) = $var)
    };
    (@ARGS_IDENT ( $($name:ident : $arg:tt),* ) = $var:expr) => {
        if let [$($name),*] = $var {
            #[allow(unused_parens)]
            match ($(match_op!($arg = $name.as_ref())),*) {
                ($(Some($name)),*) => Some(($($name),*)),
                _ => None,
            }
        } else {
            None
        }

    };
    (@GET_VARIANT not) => { Operator::Not };
    (@GET_VARIANT =) => { Operator::Eq };
    (@GET_VARIANT ite) => { Operator::Ite };
}

// Macros can only be used after they're declared, so we can't put this test in the "tests" module,
// as that module is declared in the top of the file. Instead of moving the module delcaration to
// after the macro declaration, it's easier to just bring this single test here.
#[cfg(test)]
#[test]
fn test_match_op() {
    use crate::parser::tests::{parse_term, EqByValue};

    let term = parse_term("(= (= (not false) (= true false)) (not true))");
    let ((a, (b, c)), d) = match_op!((= (= (not a) (= b c)) (not d)) = &term).unwrap();
    EqByValue::eq(a, &terminal!(bool false));
    EqByValue::eq(b, &terminal!(bool true));
    EqByValue::eq(c, &terminal!(bool false));
    EqByValue::eq(d, &terminal!(bool true));

    let term = parse_term("(ite (not true) (- 2 2) (* 1 5))");
    let (a, b, c) = match_op!((ite (not a) b c) = &term).unwrap();
    EqByValue::eq(a, &terminal!(bool true));
    EqByValue::eq(
        b,
        &Term::Op(
            Operator::Sub,
            vec![
                ByRefRc::new(terminal!(int 2)),
                ByRefRc::new(terminal!(int 2)),
            ],
        ),
    );
    EqByValue::eq(
        c,
        &Term::Op(
            Operator::Mult,
            vec![
                ByRefRc::new(terminal!(int 1)),
                ByRefRc::new(terminal!(int 5)),
            ],
        ),
    );
}

mod rules {
    use super::*;
    use std::collections::HashSet;

    /// Converts a `bool` into an `Option<()>`.
    fn to_option(b: bool) -> Option<()> {
        match b {
            true => Some(()),
            false => None,
        }
    }

    fn get_single_term_from_command(command: &ProofCommand) -> Option<&ByRefRc<Term>> {
        match command {
            ProofCommand::Assume(term) => Some(term),
            ProofCommand::Step { clause, .. } if clause.len() == 1 => Some(&clause[0]),
            _ => None,
        }
    }

    pub fn not_not(clause: &[ByRefRc<Term>], _: Vec<&ProofCommand>, _: &[ProofArg]) -> Option<()> {
        if clause.len() != 2 {
            return None;
        }
        let p = match_op!((not (not (not p))) = clause[0].as_ref())?;
        let q = clause[1].as_ref();
        to_option(p == q)
    }

    pub fn equiv_pos1(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() != 3 {
            return None;
        }
        let (phi_1, phi_2) = match_op!((not (= phi_1 phi_2)) = clause[0].as_ref())?;
        to_option(
            phi_1 == clause[1].as_ref() && phi_2 == match_op!((not phi_2) = clause[2].as_ref())?,
        )
    }

    pub fn equiv_pos2(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() != 3 {
            return None;
        }
        let (phi_1, phi_2) = match_op!((not (= phi_1 phi_2)) = clause[0].as_ref())?;
        to_option(
            phi_1 == match_op!((not phi_1) = clause[1].as_ref())? && phi_2 == clause[2].as_ref(),
        )
    }

    pub fn eq_reflexive(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() == 1 {
            let (a, b) = match_op!((= a b) = clause[0].as_ref())?;
            to_option(a == b)
        } else {
            None
        }
    }

    pub fn eq_transitive(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        /// Recursive function to find a transitive chain given a conclusion equality and a series
        /// of premise equalities.
        fn find_chain(conclusion: (&Term, &Term), premises: &mut [(&Term, &Term)]) -> Option<()> {
            // When the conclusion is of the form (= a a), it is trivially valid
            if conclusion.0 == conclusion.1 {
                return Some(());
            }

            // Find in the premises, if it exists, an equality such that one of its terms is equal
            // to the first term in the conclusion. Possibly reorder this equality so the matching
            // term is the first one
            let (index, eq) = premises.iter().enumerate().find_map(|(i, &(t, u))| {
                if t == conclusion.0 {
                    Some((i, (t, u)))
                } else if u == conclusion.0 {
                    Some((i, (u, t)))
                } else {
                    None
                }
            })?;

            // We remove the found equality by swapping it with the first element in `premises`.
            // The new premises will then be all elements after the first
            premises.swap(0, index);

            // The new conclusion will be the terms in the conclusion and the found equality that
            // didn't match. For example, if the conclusion was (= a d) and we found in the
            // premises (= a b), the new conclusion will be (= b d)
            find_chain((eq.1, conclusion.1), &mut premises[1..])
        }

        if clause.len() < 3 {
            return None;
        }

        // The last term in clause should be an equality, and it will be the conclusion of the
        // transitive chain
        let last_term = clause.last().unwrap().as_ref();
        let conclusion = match_op!((= t u) = last_term)?;

        // The first `clause.len()` - 1 terms in the clause must be a sequence of inequalites, and
        // they will be the premises of the transitive chain
        let mut premises = Vec::with_capacity(clause.len() - 1);
        for term in &clause[..clause.len() - 1] {
            let (t, u) = match_op!((not (= t u)) = term.as_ref())?;
            premises.push((t, u));
        }

        find_chain(conclusion, &mut premises)
    }

    pub fn eq_congruent(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() < 2 {
            return None;
        }

        // The first `clause.len()` - 1 terms in the clause must be a sequence of inequalites
        let mut ts = Vec::new();
        let mut us = Vec::new();
        for term in &clause[..clause.len() - 1] {
            let (t, u) = match_op!((not (= t u)) = term.as_ref())?;
            ts.push(t);
            us.push(u);
        }

        // The final term in the clause must be an equality of two function applications, whose
        // arguments are the terms in the previous inequalities
        match match_op!((= f g) = clause.last().unwrap().as_ref())? {
            (Term::App(f, f_args), Term::App(g, g_args)) => {
                if f != g || f_args.len() != ts.len() {
                    return None;
                }
                for i in 0..ts.len() {
                    let expected = (f_args[i].as_ref(), g_args[i].as_ref());
                    if expected != (ts[i], us[i]) && expected != (us[i], ts[i]) {
                        return None;
                    }
                }
                Some(())
            }
            _ => None,
        }
    }

    pub fn distinct_elim(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() != 1 {
            return None;
        }

        let (distinct_term, second_term) = match_op!((= a b) = clause[0].as_ref())?;
        let distinct_args = match distinct_term {
            Term::Op(Operator::Distinct, args) => args,
            _ => return None,
        };
        match distinct_args.as_slice() {
            [] | [_] => unreachable!(),
            [a, b] => {
                let got: (&Term, &Term) = match_op!((not (= x y)) = second_term)?;
                to_option(got == (a, b) || got == (b, a))
            }
            args => {
                if args[0].sort() == Term::BOOL_SORT {
                    // If there are more than two boolean arguments to the distinct operator, the
                    // second term must be "false"
                    return match second_term {
                        Term::Terminal(Terminal::Var(Identifier::Simple(s), _)) if s == "false" => {
                            Some(())
                        }
                        _ => None,
                    };
                }
                let got = match second_term {
                    Term::Op(Operator::And, args) => args,
                    _ => return None,
                };
                let mut k = 0;
                for i in 0..args.len() {
                    for j in i + 1..args.len() {
                        let (a, b) = (args[i].as_ref(), args[j].as_ref());
                        let got: (&Term, &Term) = match_op!((not (= x y)) = got[k].as_ref())?;
                        to_option(got == (a, b) || got == (b, a))?;
                        k += 1;
                    }
                }
                Some(())
            }
        }
    }

    pub fn resolution(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        /// Removes all leading negations in a term and returns how many there were.
        fn remove_negations(mut term: &Term) -> (u32, &Term) {
            let mut n = 0;
            while let Some(t) = match_op!((not t) = term) {
                term = t;
                n += 1;
            }
            (n, term)
        }

        // This set represents the current working clause, where (n, t) represents the term t with
        // n leading negations.
        let mut working_clause: HashSet<(u32, &Term)> = HashSet::new();

        // For every term t in each premise, we check if (not t) is in the working clause, and if
        // it is, we remove it. If t is of the form (not u), we do the same for u. If neither one
        // was removed, we insert t into the working clause.
        for command in premises.into_iter() {
            let premise_clause = match command {
                // "assume" premises are interpreted as a clause with a single term
                ProofCommand::Assume(term) => std::slice::from_ref(term),
                ProofCommand::Step { clause, .. } => &clause,
            };
            for term in premise_clause {
                let (n, inner) = remove_negations(term.as_ref());

                // Remove the entry for (n - 1, inner) if it exists
                if !(n > 0 && working_clause.remove(&(n - 1, inner))) {
                    // If it didn't exist, try the same for (n + 1, inner)
                    if !working_clause.remove(&(n + 1, inner)) {
                        // If neither entry exists, insert (n, inner)
                        working_clause.insert((n, inner));
                    }
                }
            }
        }

        // At the end, we expect the working clause to be equal to the conclusion clause
        let clause: HashSet<_> = clause
            .iter()
            .map(|t| remove_negations(t.as_ref()))
            .collect();

        to_option(working_clause == clause)
    }

    pub fn and(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if premises.len() != 1 || clause.len() != 1 {
            return None;
        }
        let and_term = get_single_term_from_command(premises[0])?;
        let and_contents = match and_term.as_ref() {
            Term::Op(Operator::And, args) => args,
            _ => return None,
        };

        to_option(and_contents.iter().any(|t| t == &clause[0]))
    }

    pub fn or(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if premises.len() != 1 {
            return None;
        }
        let or_term = get_single_term_from_command(premises[0])?;
        let or_contents = match or_term.as_ref() {
            Term::Op(Operator::Or, args) => args,
            _ => return None,
        };

        to_option(or_contents == clause)
    }

    pub fn ite1(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if premises.len() != 1 || clause.len() != 2 {
            return None;
        }
        let premise_term = get_single_term_from_command(premises[0])?;
        let (psi_1, _, psi_3) = match_op!((ite psi_1 psi_2 psi_3) = premise_term.as_ref())?;

        to_option(psi_1 == clause[0].as_ref() && psi_3 == clause[1].as_ref())
    }

    pub fn ite2(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if premises.len() != 1 || clause.len() != 2 {
            return None;
        }
        let premise_term = get_single_term_from_command(premises[0])?;
        let (psi_1, psi_2, _) = match_op!((ite psi_1 psi_2 psi_3) = premise_term.as_ref())?;

        to_option(
            psi_1 == match_op!((not psi_1) = clause[0].as_ref())? && psi_2 == clause[1].as_ref(),
        )
    }

    pub fn ite_intro(
        clause: &[ByRefRc<Term>],
        _: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if clause.len() != 1 {
            return None;
        }
        let (root_term, us) = match_op!((= t us) = clause[0].as_ref())?;
        let ite_terms: Vec<_> = root_term
            .subterms()
            .filter_map(|term| match_op!((ite a b c) = term))
            .collect();

        // "us" must be a conjunction where the first term is the root term
        let us = match us {
            Term::Op(Operator::And, args) => args,
            _ => return None,
        };
        if ite_terms.len() != us.len() - 1 || us[0].as_ref() != root_term {
            return None;
        }

        // We assume that the "ite" terms appear in the conjunction in the same order as they
        // appear as subterms of the root term
        for (s_i, u_i) in ite_terms.iter().zip(&us[1..]) {
            let (cond, (r1, s1), (r2, s2)) =
                match_op!((ite cond (= r1 s1) (= r2 s2)) = u_i.as_ref())?;

            // s_i == s1 == s2 == (ite cond r1 r2)
            let is_valid =
                (cond, r1, r2) == *s_i && s1 == s2 && match_op!((ite a b c) = s1) == Some(*s_i);

            if !is_valid {
                return None;
            }
        }
        Some(())
    }

    pub fn contraction(
        clause: &[ByRefRc<Term>],
        premises: Vec<&ProofCommand>,
        _: &[ProofArg],
    ) -> Option<()> {
        if premises.len() != 1 {
            return None;
        }

        let premise_clause: &[_] = match premises[0] {
            ProofCommand::Step { clause, .. } => &clause,
            _ => return None,
        };

        // This set will be populated with the terms we enconter as we iterate through the premise
        let mut encountered = HashSet::<&Term>::with_capacity(premise_clause.len());
        let mut clause_iter = clause.iter();

        for t in premise_clause {
            // `HashSet::insert` returns true if the inserted element was not in the set
            let is_new_term = encountered.insert(t.as_ref());

            // If the term in the premise clause has not been encountered before, we advance the
            // conclusion clause iterator, and check if its next term is the encountered term
            if is_new_term && clause_iter.next() != Some(t) {
                return None;
            }
        }

        // At the end, the conclusion clause iterator must be empty, meaning all terms in the
        // conclusion are in the premise
        to_option(clause_iter.next().is_none())
    }
}
