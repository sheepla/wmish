use crate::completion::WmiHelper;
use crate::parser::{Command, OutputFormat, parse_command};
use crate::wmi::{
    WmiClient, WmiProvider, WmiResult, get_property, get_property_names, variant_to_string,
};
use rustyline::{Config, Editor};
use std::sync::{Arc, Mutex};

pub struct Shell {
    client: Arc<Mutex<Box<dyn WmiProvider>>>,
    namespace: Arc<Mutex<String>>,
    format: OutputFormat,
}

impl Shell {
    pub fn new() -> windows::core::Result<Self> {
        let namespace = r#"ROOT\CIMV2"#.to_string();
        let client = WmiClient::connect(&namespace)?;
        Ok(Self {
            client: Arc::new(Mutex::new(Box::new(client))),
            namespace: Arc::new(Mutex::new(namespace)),
            format: OutputFormat::Table,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

    pub fn execute_command(&mut self, cmd: Command) -> windows::core::Result<()> {
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
            Command::Select(query) => {
                self.execute_query(&query)?;
            }
            Command::Format(f) => {
                self.format = f;
            }
            Command::Call { method, target } => {
                println!("Calling method {} on {} (Not implemented)", method, target);
            }
            Command::Exit => std::process::exit(0),
        }
        Ok(())
    }

    pub fn execute_query(&self, query: &str) -> windows::core::Result<()> {
        let client = self.client.lock().unwrap();
        let results = client.query(query)?;
        let it = WmiResult::new(results);
        let mut first = true;
        let mut headers = Vec::new();

        for obj in it {
            let obj = obj?;
            if first {
                headers = get_property_names(&obj)?;
                if self.format == OutputFormat::Csv {
                    println!("{}", headers.join(","));
                }
                first = false;
            }

            let mut row = Vec::new();
            for header in &headers {
                let val = get_property(&obj, header)?;
                row.push(variant_to_string(&val));
            }

            match self.format {
                OutputFormat::Csv => println!("{}", row.join(",")),
                OutputFormat::Table => println!("{}", row.join("\t")),
                OutputFormat::Json => {
                    // TODO rewrite with serde and print pretty JSON
                    print!("{{");
                    for (i, h) in headers.iter().enumerate() {
                        print!(
                            "\"{}\": \"{}\"{}",
                            h,
                            row[i],
                            if i == headers.len() - 1 { "" } else { ", " }
                        );
                    }
                    println!("}}");
                }
            }
        }
        Ok(())
    }
}
