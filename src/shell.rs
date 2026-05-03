use crate::completion::WmiHelper;
use crate::errors::AppError;
use crate::parser::{Command, OutputFormat, parse_command};
use crate::wmi::{
    WmiClient, WmiProvider, WmiResult, get_property, get_property_names, variant_to_string, get_object_text,
};
use rustyline::{Config, Editor};
use std::sync::{Arc, Mutex};
use tabled::Table;
use tabled::settings::Style;

pub struct Shell {
    client: Arc<Mutex<Box<dyn WmiProvider>>>,
    namespace: Arc<Mutex<String>>,
    format: OutputFormat,
}

impl Shell {
    pub fn new() -> std::result::Result<Self, AppError> {
        let namespace = r#"ROOT\CIMV2"#.to_string();
        let client = WmiClient::connect(&namespace)?;
        Ok(Self {
            client: Arc::new(Mutex::new(Box::new(client))),
            namespace: Arc::new(Mutex::new(namespace)),
            format: OutputFormat::Table,
        })
    }

    pub fn set_format(&mut self, format: OutputFormat) {
        self.format = format;
    }

    pub fn run(&mut self) -> std::result::Result<(), AppError> {
        let config = Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .build();
        let mut rl: Editor<WmiHelper, rustyline::history::DefaultHistory> =
            Editor::with_config(config)?;

        let h = WmiHelper::new(Arc::clone(&self.client), Arc::clone(&self.namespace));
        rl.set_helper(Some(h));

        loop {
            let ns = self.namespace.lock().unwrap().clone();
            let prompt = format!("{} >> ", ns);
            let line = match rl.readline(&prompt) {
                Ok(line) => line,
                Err(rustyline::error::ReadlineError::Interrupted) => break,
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(e) => return Err(e.into()),
            };
            rl.add_history_entry(line.as_str())?;

            if line.trim().is_empty() {
                continue;
            }

            match parse_command(&line) {
                Ok((_, cmd)) => {
                    if let Err(e) = self.execute_command(cmd) {
                        eprintln!("Error: {}", e);
                    }
                }
                Err(_) => {
                    if line.trim().to_uppercase().starts_with("SELECT") {
                        if let Err(e) = self.execute_query(&line) {
                            eprintln!("Error: {}", e);
                        }
                    } else {
                        eprintln!("Invalid command");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn execute_command(&mut self, cmd: Command) -> std::result::Result<(), AppError> {
        match cmd {
            Command::Namespace(ns) => {
                let new_client = WmiClient::connect(&ns)?;
                let mut client_lock = self.client.lock().unwrap();
                *client_lock = Box::new(new_client);
                let mut ns_lock = self.namespace.lock().unwrap();
                *ns_lock = ns;
            }
            Command::Classes => {
                let client = self.client.lock().unwrap();
                let results = client.list_classes()?;
                let it = WmiResult::new(results);
                for obj in it {
                    let obj = obj?;
                    let name = get_property(&obj, "__CLASS")?;
                    println!("{}", variant_to_string(&name));
                }
            }
            Command::Show(class) => {
                let client = self.client.lock().unwrap();
                let obj = client.get_class(&class)?;
                let names = get_property_names(&obj)?;
                for name in names {
                    println!("{}", name);
                }
            }
            Command::MOF(class) => {
                let client = self.client.lock().unwrap();
                let obj = client.get_class(&class)?;
                let mof = get_object_text(&obj)?;
                println!("{}", mof);
            }
            Command::Select(query) => {
                self.execute_query(&query)?;
            }
            Command::Format(f) => {
                self.format = f;
            }
            Command::Call { method, target, .. } => {
                println!("Calling method {} on {} (Not implemented)", method, target);
            }
            Command::Exit => std::process::exit(0),
        }
        Ok(())
    }

    pub fn execute_query(&self, query: &str) -> std::result::Result<(), AppError> {
        let client = self.client.lock().unwrap();
        let results = client.query(query)?;
        let it = WmiResult::new(results);
        
        let mut rows = Vec::new();
        let mut headers = Vec::new();
        let mut first = true;

        for obj in it {
            let obj = obj?;
            if first {
                headers = get_property_names(&obj)?;
                first = false;
            }

            let mut row = Vec::new();
            for header in &headers {
                let val = get_property(&obj, header)?;
                row.push(variant_to_string(&val));
            }
            rows.push(row);
        }

        if rows.is_empty() {
            return Ok(());
        }

        match self.format {
            OutputFormat::Csv => {
                println!("{}", headers.join(","));
                for row in rows {
                    println!("{}", row.join(","));
                }
            }
            OutputFormat::Table | OutputFormat::Ascii | OutputFormat::Markdown => {
                let mut data = Vec::new();
                data.push(headers);
                for row in rows {
                    data.push(row);
                }

                let mut table = Table::from_iter(data);
                match self.format {
                    OutputFormat::Table => table.with(Style::sharp()),
                    OutputFormat::Ascii => table.with(Style::psql()),
                    OutputFormat::Markdown => table.with(Style::markdown()),
                    _ => unreachable!(),
                };
                println!("{}", table);
            }
            OutputFormat::Json => {
                let mut json_arr = Vec::new();
                for row in rows {
                    let mut map = serde_json::Map::new();
                    for (i, header) in headers.iter().enumerate() {
                        map.insert(header.clone(), serde_json::Value::String(row[i].clone()));
                    }
                    json_arr.push(serde_json::Value::Object(map));
                }
                println!("{}", serde_json::to_string_pretty(&serde_json::Value::Array(json_arr))?);
            }
        }
        Ok(())
    }
}
