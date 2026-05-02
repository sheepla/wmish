use rustyline::completion::{Completer, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::sync::{Arc, Mutex};
use crate::wmi::WmiProvider;

pub struct WmiHelper {
    pub client: Arc<Mutex<Box<dyn WmiProvider>>>,
    pub namespace: Arc<Mutex<String>>,
    commands: Vec<String>,
}

impl WmiHelper {
    pub fn new(client: Arc<Mutex<Box<dyn WmiProvider>>>, namespace: Arc<Mutex<String>>) -> Self {
        Self {
            client,
            namespace,
            commands: vec![
                "NAMESPACE".to_string(),
                "CLASSES".to_string(),
                "SHOW".to_string(),
                "SELECT".to_string(),
                "FORMAT".to_string(),
                "CALL".to_string(),
                "EXIT".to_string(),
                "QUIT".to_string(),
            ],
        }
    }
}

impl Completer for WmiHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut candidates = Vec::new();
        let up_line = line.to_uppercase();

        if !line.contains(' ') {
            for cmd in &self.commands {
                if cmd.starts_with(&up_line) {
                    candidates.push(Pair {
                        display: cmd.clone(),
                        replacement: cmd.clone(),
                    });
                }
            }
            return Ok((0, candidates));
        }

        Ok((0, candidates))
    }
}

impl Helper for WmiHelper {}

impl Hinter for WmiHelper {
    type Hint = String;
}

impl Highlighter for WmiHelper {}

impl Validator for WmiHelper {}
