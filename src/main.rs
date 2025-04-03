use std::env;
use std::error::Error;
use std::path::PathBuf;
use dotenv::dotenv;

// Import our modules
use roblox_mcp::{
    cli,
    gemini_api::GeminiClient,
    roblox::{self, json_to_weakdom, Modification, write_roblox_file},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Set up CLI
    let matches = cli::build_cli().get_matches();

    // Get the filepath from the command-line arguments
    if let Some(filepath) = matches.get_one::<PathBuf>("filepath") {
        println!("Input filepath: {}", filepath.display());

        // Parse the XML file into a Roblox place
        let mut place = roblox::parse_roblox_file(filepath)?;
        println!("Successfully parsed place file!");

        // Get the API key either from command line arguments or environment variable
        let api_key = matches
            .get_one::<String>("api-key")
            .map(|s| s.to_string())
            .or_else(|| env::var("GEMINI_API_KEY").ok())
            .ok_or("Gemini API key not provided. Use --api-key option or set GEMINI_API_KEY environment variable")?;

        // Get the user prompt
        let prompt = matches
            .get_one::<String>("prompt")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Analyze this Roblox place structure".to_string());

        println!("Using prompt: {}", prompt);

        // Get the context file if provided
        let context = matches
            .get_one::<PathBuf>("context")
            .and_then(|path| {
                if path.extension().map_or(false, |ext| ext == "md") {
                    match std::fs::read_to_string(path) {
                        Ok(content) => {
                            println!("Loaded context from: {}", path.display());
                            Some(content)
                        },
                        Err(e) => {
                            eprintln!("Error reading context file: {}", e);
                            None
                        }
                    }
                } else {
                    eprintln!("Context file must be a markdown (.md) file");
                    None
                }
            });

        // Create Gemini client and generate content
        let client = GeminiClient::flash(api_key);
        let response = client.generate_content(&prompt, &place, 8000, 0.8, context).await?;

        // Process the response
        let text = GeminiClient::extract_text(&response).unwrap();
        println!("Gemini API Response:");
        println!("{}", text);

        let json: Modification = serde_json::from_str(&text).unwrap();
        let root_ref = place.root().referent();
        json_to_weakdom(&mut place, &json, root_ref)?;
        write_roblox_file("output.rbxlx", &place)?;
    }

    Ok(())
}
