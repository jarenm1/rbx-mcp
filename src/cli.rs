use clap::{Arg, Command};
use std::path::PathBuf;

pub fn build_cli() -> Command {
    Command::new("roblox-mcp")
        .version("0.1.0")
        .author("Author")
        .about("Roblox MCP tool")
        .arg(
            Arg::new("filepath")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Input file path")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("api-key")
                .short('k')
                .long("api-key")
                .value_name("KEY")
                .help("Gemini API key (can also be provided via GEMINI_API_KEY env variable)")
                .required(false),
        )
        .arg(
            Arg::new("prompt")
                .short('p')
                .long("prompt")
                .value_name("PROMPT")
                .help("Custom prompt to send to Gemini along with the place data")
                .default_value("Analyze this Roblox place structure")
                .required(false),
        )
        .arg(
            Arg::new("context")
                .short('c')
                .long("context")
                .value_name("FILE")
                .help("Context file path (markdown .md)")
                .required(false),
        )
}
