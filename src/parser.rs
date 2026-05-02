use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{alphanumeric1, multispace0, multispace1},
    combinator::{map, rest},
    sequence::preceded,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Namespace(String),
    Classes,
    Show(String),
    MOF(String),
    Select(String),
    Format(OutputFormat),
    Call {
        method: String,
        args: Vec<(String, String)>,
        target: String, // ClassName or Query
    },
    Exit,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OutputFormat {
    Csv,
    Table,
    Json,
    Ascii,
    Markdown,
}

pub fn parse_command(input: &str) -> IResult<&str, Command> {
    alt((
        parse_exit,
        parse_namespace,
        parse_classes,
        parse_show,
        parse_mof,
        parse_select,
        parse_format,
        parse_call,
    ))
    .parse(input)
}

fn parse_exit(input: &str) -> IResult<&str, Command> {
    map(alt((tag_no_case("EXIT"), tag_no_case("QUIT"))), |_| {
        Command::Exit
    })
    .parse(input)
}

fn parse_namespace(input: &str) -> IResult<&str, Command> {
    map(
        preceded((tag_no_case("NAMESPACE"), multispace1), rest),
        |ns: &str| Command::Namespace(ns.trim().to_string()),
    )
    .parse(input)
}

fn parse_classes(input: &str) -> IResult<&str, Command> {
    map(tag_no_case("CLASSES"), |_| Command::Classes).parse(input)
}

fn parse_show(input: &str) -> IResult<&str, Command> {
    map(
        preceded((tag_no_case("SHOW"), multispace1), rest),
        |class: &str| Command::Show(class.trim().to_string()),
    )
    .parse(input)
}

fn parse_mof(input: &str) -> IResult<&str, Command> {
    map(
        preceded((tag_no_case("MOF"), multispace1), rest),
        |class: &str| Command::MOF(class.trim().to_string()),
    )
    .parse(input)
}

fn parse_select(input: &str) -> IResult<&str, Command> {
    // Better SELECT parsing: it should capture the entire WQL query correctly
    let (input, _) = tag_no_case("SELECT").parse(input)?;
    let (_input, rest_of_query) = rest.parse(input)?;
    
    let full_query = format!("SELECT {}", rest_of_query.trim());
    Ok(("", Command::Select(full_query)))
}

fn parse_format(input: &str) -> IResult<&str, Command> {
    map(
        preceded(
            (tag_no_case("FORMAT"), multispace1),
            alt((
                map(tag_no_case("CSV"), |_| OutputFormat::Csv),
                map(tag_no_case("TABLE"), |_| OutputFormat::Table),
                map(tag_no_case("JSON"), |_| OutputFormat::Json),
                map(tag_no_case("ASCII"), |_| OutputFormat::Ascii),
                map(tag_no_case("MARKDOWN"), |_| OutputFormat::Markdown),
            )),
        ),
        Command::Format,
    )
    .parse(input)
}

fn parse_call(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag_no_case("CALL").parse(input)?;
    let (input, _) = multispace1::<&str, nom::error::Error<&str>>.parse(input)?;
    let (input, method) = alphanumeric1.parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    
    // Check for WITH
    let mut temp_input = input;
    let mut target = "";
    if let Ok((i, _)) = multispace1::<&str, nom::error::Error<&str>>.parse(temp_input) {
        if let Ok((i, _)) = tag_no_case::<&str, &str, nom::error::Error<&str>>("WITH").parse(i) {
            if let Ok((i, _)) = multispace1::<&str, nom::error::Error<&str>>.parse(i) {
                temp_input = i;
                target = temp_input.trim();
            }
        }
    }

    Ok(("", Command::Call {
        method: method.to_string(),
        args: vec![],
        target: target.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exit() {
        assert_eq!(parse_command("EXIT"), Ok(("", Command::Exit)));
        assert_eq!(parse_command("quit"), Ok(("", Command::Exit)));
    }

    #[test]
    fn test_parse_namespace() {
        assert_eq!(
            parse_command(r#"NAMESPACE ROOT\CIMV2"#),
            Ok(("", Command::Namespace(r#"ROOT\CIMV2"#.to_string())))
        );
    }

    #[test]
    fn test_parse_classes() {
        assert_eq!(parse_command("CLASSES"), Ok(("", Command::Classes)));
    }

    #[test]
    fn test_parse_show() {
        assert_eq!(
            parse_command("SHOW Win32_Process"),
            Ok(("", Command::Show("Win32_Process".to_string())))
        );
    }

    #[test]
    fn test_parse_select() {
        let query = "SELECT * FROM Win32_Process";
        assert_eq!(
            parse_command(query),
            Ok(("", Command::Select(query.to_string())))
        );
    }

    #[test]
    fn test_parse_format() {
        assert_eq!(
            parse_command("FORMAT JSON"),
            Ok(("", Command::Format(OutputFormat::Json)))
        );
        assert_eq!(
            parse_command("format csv"),
            Ok(("", Command::Format(OutputFormat::Csv)))
        );
    }

    #[test]
    fn test_parse_call() {
        assert_eq!(
            parse_command("CALL Create WITH Win32_Process"),
            Ok((
                "",
                Command::Call {
                    method: "Create".to_string(),
                    target: "Win32_Process".to_string()
                }
            ))
        );
    }
}
