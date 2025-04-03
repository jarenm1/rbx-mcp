use reqwest;
use serde_json::{json, Value};
use std::error::Error;

/// Structure to hold Gemini API configuration
pub struct GeminiClient {
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Self {
        GeminiClient { api_key, model }
    }

    /// Create a default client with the gemini-pro model
    pub fn default(api_key: String) -> Self {
        GeminiClient {
            api_key,
            model: "gemini-pro".to_string(),
        }
    }

    /// Create a client with the flash model
    pub fn flash(api_key: String) -> Self {
        GeminiClient {
            api_key,
            model: "gemini-2.0-flash".to_string(),
        }
    }

    /// Send a request to the Gemini API
    pub async fn generate_content(
        &self,
        prompt: &str,
        place: &impl std::fmt::Debug,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<Value, Box<dyn Error>> {
        // Create a request payload for Gemini
        let request_body = json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": format!("RESPOND ONLY WITH RAW JSON, NO MARKDOWN CODE BLOCKS, NO BACKTICKS. DO NOT INCLUDE ```json AT THE BEGINNING OR ``` AT THE END. Your response must be a pure JSON document that can be directly parsed by a JSON parser. {}: {:?}", prompt, place)
                        },
                        {
                            "text": format!("IMPORTANT: DO NOT wrap your response in code blocks or any other formatting. ONLY RETURN JSON in this exact format: {}", example_prompt())
                        },
                        {
                            "text": format!("RESPOND ONLY WITH ADDED INSTANCES. DO NOT PROVIDE ANYTHING ELSE. {}", documentation_prompt())
                        }
                    ]
                }
            ],
            "generationConfig": {
                "temperature": temperature,
                "maxOutputTokens": max_tokens,
                "response_mime_type": "application/json"
            }
        });

        // Basic request setup for Gemini API
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                self.model, self.api_key
            ))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            // Clone status for the error message if needed
            let _status = response.status();
            
            // Parse the response to JSON
            match response.json::<Value>().await {
                Ok(gemini_response) => Ok(gemini_response),
                Err(e) => Err(format!("Failed to parse JSON response: {}", e).into())
            }
        } else {
            let status = response.status();
            let error_body = response.text().await?;
            Err(format!("Error: HTTP {}. Details: {}", status, error_body).into())
        }
    }

    /// Extract text from Gemini response
    pub fn extract_text(response: &Value) -> Option<String> {
        response
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
    }
}


fn example_prompt() -> String {

    r#"
    {
        "add": [
            {
                "class": "Part",
                "name": "HouseBase",
                "properties": {
                    "CFrame": {
                        "type": "CFrame",
                        "value": {
                            "position": [10.0, 5.0, 0.0],
                            "rotation": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                        }
                    },
                    "Size": {
                        "type": "Vector3",
                        "value": [10.0, 5.0, 10.0]
                    },
                    "BrickColor": {
                        "type": "BrickColor",
                        "value": 194
                    }
                },
                "children": [
                    {
                        "class": "Decal",
                        "name": "Window",
                        "properties": {
                            "Texture": {
                                "type": "String",
                                "value": "rbxassetid://123456"
                            }
                        },
                        "children": []
                    }
                ]
            }
        ]
    }
    "#.to_string()
}

fn documentation_prompt() -> String {
    r#"
    You will add a Item element. This item element will have a class, this class is the type of Instance of the item.
    https://create.roblox.com/docs/reference/engine/classes/Instance 
    Each class has its own properties and can also have properties infered from other classes.
    Please correctly add the correct properties for each added item.
    "#.to_string()
}