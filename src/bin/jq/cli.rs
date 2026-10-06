use std::{fs::File, io::Read, str::FromStr};

use crate::error::AppError;
use rs_jq::jql::jql_parse;
use rs_jq::json::{json_load, value::JsonValue};
use structopt::StructOpt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum InputFormat {
    JSON,
    AUTO,
}

impl FromStr for InputFormat {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(InputFormat::JSON),
            "auto" => Ok(InputFormat::AUTO),
            _ => Err(AppError::InvalidFormat),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "jq", about = "CLI for rs-jq")]
struct Opts {
    #[structopt(short, long, default_value = "")]
    input: String,

    #[structopt(short, long, default_value = "auto")]
    format: InputFormat,

    #[structopt(short = "s", long, default_value = "")]
    script: String,
}

impl Opts {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.input == String::from("") && self.format == InputFormat::AUTO {
            return Err(AppError::InvalidFormat);
        }
        Ok(())
    }
}

fn load_input(input: &String) -> Result<Box<dyn Read>, AppError> {
    match input.as_str() {
        "" => Ok(Box::new(std::io::stdin())),
        _ => Ok(Box::new(
            File::open(&input).map_err(|_| AppError::InvalidInput)?,
        )),
    }
}

fn process(mut reader: Box<dyn Read>, opts: &Opts) -> Result<JsonValue, AppError> {
    let mut source = String::new();
    reader
        .read_to_string(&mut source)
        .map_err(|_| AppError::InvalidInput)?;

    let json_value = json_load(&source).map_err(|err| AppError::ParseError(err.to_string()))?;

    let ast = jql_parse(&opts.script).map_err(|err| AppError::ParseError(err.to_string()))?;
    println!("[DEBUG] JqlAst");
    println!("{:?}", ast);

    Ok(json_value)
}

pub fn execute() -> Result<(), AppError> {
    let opts = Opts::from_args();

    opts.validate()?;

    let reader = load_input(&opts.input)?;
    let json_value = process(reader, &opts)?;
    println!("{}", json_value);

    Ok(())
}
