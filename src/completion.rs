use rustyline::completion::{Completer, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::sync::{Arc, Mutex};
use crate::wmi::{WmiProvider, WmiResult, get_property, variant_to_string};

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
                "MOF".to_string(),
                "SELECT".to_string(),
                "FORMAT".to_string(),
                "CALL".to_string(),
                "EXIT".to_string(),
                "QUIT".to_string(),
            ],
        }
    }

    fn get_classes(&self) -> Vec<String> {
        let client = self.client.lock().unwrap();
        let mut classes = Vec::new();
        if let Ok(results) = client.list_classes() {
            let it = WmiResult::new(results);
            for obj in it {
                if let Ok(obj) = obj {
                    if let Ok(name) = get_property(&obj, "__CLASS") {
                        classes.push(variant_to_string(&name));
                    }
                }
            }
        }
        classes
    }

    fn get_properties(&self, class_name: &str) -> Vec<String> {
        let client = self.client.lock().unwrap();
        if let Ok(obj) = client.get_class(class_name) {
            if let Ok(names) = crate::wmi::get_property_names(&obj) {
                return names;
            }
        }
        Vec::new()
    }

    fn get_namespaces(&self, prefix: &str) -> Vec<String> {
        let mut namespaces = Vec::new();
        let up_prefix = prefix.to_uppercase();

        if "ROOT".starts_with(&up_prefix) && !up_prefix.contains('\\') {
            namespaces.push("ROOT".to_string());
        }

        // Determine which namespace to query
        let (query_ns, parent_path) = if let Some(idx) = prefix.rfind('\\').or(prefix.rfind('/')) {
            let p = &prefix[..idx];
            if p.is_empty() {
                ("ROOT".to_string(), "\\".to_string())
            } else {
                (p.to_string(), format!("{}\\", p))
            }
        } else {
            // Relative to current
            (self.namespace.lock().unwrap().clone(), "".to_string())
        };

        // Check if we can use the current client or need a new one
        let current_ns = self.namespace.lock().unwrap().clone();
        
        let results = if query_ns.to_uppercase() == current_ns.to_uppercase() {
            let client = self.client.lock().unwrap();
            client.query("SELECT Name FROM __NAMESPACE")
        } else {
            // Connect to the specific namespace to get its REAL children
            match crate::wmi::WmiClient::connect(&query_ns) {
                Ok(temp_client) => temp_client.query("SELECT Name FROM __NAMESPACE"),
                Err(_) => return namespaces, // Invalid namespace, no children
            }
        };

        if let Ok(res) = results {
            let it = WmiResult::new(res);
            for obj in it {
                if let Ok(obj) = obj {
                    if let Ok(name_var) = get_property(&obj, "Name") {
                        let name = variant_to_string(&name_var);
                        let full_name = format!("{}{}", parent_path, name);
                        if full_name.to_uppercase().starts_with(&up_prefix) {
                            namespaces.push(full_name);
                        }
                    }
                }
            }
        }
        
        namespaces
    }
}

impl Completer for WmiHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut candidates = Vec::new();
        let up_line = line.to_uppercase();
        
        // Find the word being completed at pos
        let prefix = &line[..pos];
        // Split by space or comma, but NOT by slashes for namespace completion
        let last_word = prefix.split(|c: char| c == ' ' || c == ',').last().unwrap_or("");
        let word_start = pos - last_word.len();
        let up_last_word = last_word.to_uppercase();

        // Check the first word of the line to determine command
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() || (words.len() == 1 && pos <= words[0].len() + (line.len() - line.trim_start().len())) {
            // Completing the command itself
            for cmd in &self.commands {
                if cmd.starts_with(&up_last_word) {
                    candidates.push(Pair {
                        display: cmd.clone(),
                        replacement: cmd.clone() + " ",
                    });
                }
            }
            return Ok((word_start, candidates));
        }

        let first_word = words[0].to_uppercase();

        match first_word.as_str() {
            "NAMESPACE" => {
                let arg = if line.len() > 10 { &line[10..pos] } else { "" };
                for ns in self.get_namespaces(arg) {
                    candidates.push(Pair {
                        display: ns.clone(),
                        replacement: ns,
                    });
                }
                return Ok((10, candidates));
            }
            "SHOW" | "MOF" => {
                for class in self.get_classes() {
                    if class.to_uppercase().starts_with(&up_last_word) {
                        candidates.push(Pair {
                            display: class.clone(),
                            replacement: class,
                        });
                    }
                }
                return Ok((word_start, candidates));
            }
            "SELECT" => {
                // Find "FROM" and "WHERE" in the line
                let from_match = up_line.find(" FROM ");
                let where_match = up_line.find(" WHERE ");
                
                if let Some(f_idx) = from_match {
                    let rest_after_from = &line[f_idx + 6..];
                    let class_name = rest_after_from.split_whitespace().next().unwrap_or("");

                    if pos <= f_idx + 1 {
                        // Cursor is between SELECT and FROM -> Complete properties
                        if !class_name.is_empty() {
                            for prop in self.get_properties(class_name) {
                                if prop.to_uppercase().starts_with(&up_last_word) {
                                    candidates.push(Pair {
                                        display: prop.clone(),
                                        replacement: prop,
                                    });
                                }
                            }
                            return Ok((word_start, candidates));
                        }
                    } else if let Some(w_idx) = where_match {
                        if pos > w_idx + 7 {
                            // Cursor is after WHERE -> Complete properties and operators
                            if !class_name.is_empty() {
                                for prop in self.get_properties(class_name) {
                                    if prop.to_uppercase().starts_with(&up_last_word) {
                                        candidates.push(Pair {
                                            display: prop.clone(),
                                            replacement: prop,
                                        });
                                    }
                                }
                            }
                            
                            let operators = vec![
                                "=", "<>", "!=", ">", "<", ">=", "<=", 
                                "LIKE", "IS", "IS NOT", "ISA", 
                                "AND", "OR", "NOT"
                            ];
                            for op in operators {
                                if op.starts_with(&up_last_word) {
                                    candidates.push(Pair {
                                        display: op.to_string(),
                                        replacement: op.to_string() + " ",
                                    });
                                }
                            }
                            return Ok((word_start, candidates));
                        } else if pos > f_idx + 6 && pos <= w_idx + 1 {
                             // Cursor is between FROM and WHERE -> Complete class name
                             for class in self.get_classes() {
                                if class.to_uppercase().starts_with(&up_last_word) {
                                    candidates.push(Pair {
                                        display: class.clone(),
                                        replacement: class,
                                    });
                                }
                            }
                            return Ok((word_start, candidates));
                        }
                    } else if pos > f_idx + 6 {
                        // Cursor is after FROM but no WHERE yet
                        let after_from = &up_line[f_idx + 6..pos];
                        if !after_from.contains(' ') {
                             for class in self.get_classes() {
                                if class.to_uppercase().starts_with(&up_last_word) {
                                    candidates.push(Pair {
                                        display: class.clone(),
                                        replacement: class,
                                    });
                                }
                            }
                        } else {
                            let keywords = vec!["WHERE", "ORDER BY"];
                            for kw in keywords {
                                if kw.starts_with(&up_last_word) {
                                    candidates.push(Pair {
                                        display: kw.to_string(),
                                        replacement: kw.to_string() + " ",
                                    });
                                }
                            }
                        }
                        return Ok((word_start, candidates));
                    }
                }

                // Default SELECT suggestions
                let suggestions = vec!["*", "FROM"];
                for s in suggestions {
                    if s.starts_with(&up_last_word) {
                        candidates.push(Pair {
                            display: s.to_string(),
                            replacement: s.to_string() + " ",
                        });
                    }
                }
                return Ok((word_start, candidates));
            }
            "FORMAT" => {
                let formats = vec!["CSV", "TABLE", "JSON", "ASCII", "MARKDOWN"];
                for f in formats {
                    if f.starts_with(&up_last_word) {
                        candidates.push(Pair {
                            display: f.to_string(),
                            replacement: f.to_string(),
                        });
                    }
                }
                return Ok((word_start, candidates));
            }
            _ => {}
        }

        Ok((word_start, candidates))
    }
}

impl Helper for WmiHelper {}

impl Hinter for WmiHelper {
    type Hint = String;
}

impl Highlighter for WmiHelper {}

impl Validator for WmiHelper {}
