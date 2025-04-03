use std::env;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use dotenv::dotenv;

use roblox_mcp::cli::build_cli;
use roblox_mcp::gemini_api::GeminiClient;
use roblox_mcp::roblox::{self, write_roblox_file, Modification};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Set up CLI
    let matches = build_cli().get_matches();

    // Get the filepath from the command-line arguments
    let filepath = matches.get_one::<PathBuf>("filepath")
        .ok_or("Filepath must be provided")?;
    println!("Input filepath: {}", filepath.display());

    // Initial parse to verify the file is valid
    let _ = roblox::parse_roblox_file(filepath)?;
    println!("Successfully parsed place file!");

    // Get the API key either from command line arguments or environment variable
    let api_key = matches
        .get_one::<String>("api-key")
        .map(|s| s.to_string())
        .or_else(|| env::var("GEMINI_API_KEY").ok())
        .ok_or("Gemini API key not provided. Use --api-key option or set GEMINI_API_KEY environment variable")?;

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
                eprintln!("Context file must have .md extension");
                None
            }
        });

    // Create Gemini client
    let client = GeminiClient::flash(api_key);
    
    println!("\n===== ROBLOX MCP INTERACTIVE MODE =====");
    println!("Enter prompts to modify your Roblox place. Press Ctrl+C to exit.");

    loop {
        // Re-parse the place at the start of each loop to get fresh data
        let mut place = match roblox::parse_roblox_file(filepath) {
            Ok(place) => place,
            Err(e) => {
                eprintln!("Error parsing place file: {}", e);
                continue;
            }
        };
        
        // Ask for a prompt at each iteration
        let mut current_prompt = String::new();
        print!("\nEnter your prompt: ");
        io::stdout().flush()?;
        io::stdin().lock().read_line(&mut current_prompt)?;
        current_prompt = current_prompt.trim().to_string();
        
        // Check for exit command
        if current_prompt.to_lowercase() == "exit" || current_prompt.to_lowercase() == "quit" {
            println!("Exiting MCP interactive mode");
            break;
        }
        
        // Skip empty prompts
        if current_prompt.is_empty() {
            println!("Prompt is empty, please try again");
            continue;
        }
        
        println!("Processing prompt: {}", current_prompt);
        
        // Generate content with Gemini
        match client.generate_content(&current_prompt, &place, 8000, 0.8, context.clone()).await {
            Ok(response) => {
                // Extract and process the response
                let text_option = GeminiClient::extract_text(&response);
                match text_option {
                    Some(text) => {
                        println!("Gemini API Response:");
                        println!("{}", text);
                        
                        // Try to parse the response as JSON directly
                        match serde_json::from_str::<Modification>(&text) {
                            Ok(modification) => {
                                // Modify the place with the parsed data
                                let root_ref = place.root_ref();
                                if let Err(e) = roblox::json_to_weakdom(&mut place, &modification, root_ref) {
                                    eprintln!("Error modifying place: {}", e);
                                    continue;
                                }
                                
                                // Save by overwriting the original input file
                                if let Err(e) = write_roblox_file(&filepath, &place) {
                                    eprintln!("Error writing to input file: {}", e);
                                    continue;
                                }
                                
                                println!("Updated original file: {}", filepath.display());
                            },
                            Err(e) => {
                                eprintln!("Error parsing JSON: {}", e);
                                eprintln!("Raw response: {}", text);
                            }
                        }
                    },
                    None => {
                        eprintln!("No text found in Gemini response");
                    }
                }
            },
            Err(e) => {
                eprintln!("Error generating content: {}", e);
                continue;
            }
        }
    }

    Ok(())
}
