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
        context: Option<String>,
    ) -> Result<Value, Box<dyn Error>> {
        // Create a request payload for Gemini
        let mut request_parts = vec![
            json!({
                "text": format!("RESPOND ONLY WITH RAW JSON, NO MARKDOWN CODE BLOCKS, NO BACKTICKS. DO NOT INCLUDE ```json AT THE BEGINNING OR ``` AT THE END. Your response must be a pure JSON document that can be directly parsed by a JSON parser. {}: {:?}", prompt, place)
            }),
            json!({
                "text": format!("IMPORTANT: DO NOT wrap your response in code blocks or any other formatting. ONLY RETURN JSON in this exact format: {}", example_prompt())
            }),
            json!({
                "text": format!("RESPOND ONLY WITH ADDED INSTANCES. DO NOT PROVIDE ANYTHING ELSE. {}", documentation_prompt())
            })
        ];

        // Add context if provided
        if let Some(ctx) = context {
            request_parts.push(json!({
                "text": format!("Additional context for your consideration: {}", ctx)
            }));
        }

        let request_body = json!({
            "contents": [
                {
                    "parts": request_parts
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
                "name": "Base",
                "target_parent": "Workspace/House",
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
                    },
                    "Material": {
                        "type": "Enum",
                        "value": 1
                    },
                    "Color": {
                        "type": "Color3",
                        "value": [1.0, 1.0, 1.0]
                    }
                },
                "children": [
                    {
                        "class": "Decal",
                        "name": "Painting",
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
        "subtract": [
            "Workspace/House/Door",
            "Workspace/Tree/Window"
        ]
    }
    "#.to_string()
}

fn documentation_prompt() -> String {
    r#"
    
    You can target nested instances using path syntax with forward slashes:
    - Basic services: "Workspace", "ServerScriptService", etc.
    - Nested paths: "Workspace/Map", "Workspace/Models/House", "ReplicatedStorage/Assets/Weapons"
    - Instance names in the path MUST MATCH EXACTLY with existing instances

    YOU MUST START WITH THE HIGHEST LEVEL. i.e. "Workspace" or "ReplicatedStorage" AND INDEX TO TARGET. THIS IS REQUIRED!
    DO NOT SKIP THIS STEP.
    YOU MUST INDEX TO TARGET BASED ON THE PROVIDED DOM CONTEXT.

    You can remove instances by providing a path to the instance you want to remove in subtract.
    When asked to modify, or rewrite, remove the old instance when adding the new one.
    
    Valid target_parent examples:
    - "Workspace" - Top-level workspace (for physical objects, parts, models)
    - "ServerScriptService" - For server-side scripts
    - "Workspace/Environment" - Inside a potential folder named "Environment" in Workspace
    - "ReplicatedStorage/Weapons/Swords" - Deep nesting is supported
    - "StarterPlayer" - For StarterPlayer
    - "StarterPlayer/StarterPlayerScripts" - For scripts in StarterPlayerScripts
    - "StarterPlayer/StarterCharacter" - For scripts StarterCharacter
    - "StarterGui" - For GUI
    - "StarterPack" - For character items.

    
    Example of correctly specifying a parent:
    "class": "Part",
    "name": "Door",
    "target_parent": "Workspace/House",

    Set the run context for scripts with the correct enum.
    
    BE VERY IN DEPTH WITH WHAT IS ADDED. ADD MORE DETAIL.
    ADD MORE INSTANCES TO ADD MORE DETAIL.
    DOING MANY NESTED CHILDREN IS ALSO OK, AND MAY BE NEEDED IN SOME CASES.

    IF YOU ARE ASKED TO MODIFY SOMETHING, SET THE CORRECT target_parent BASED ON REQUEST.
    EXAMPLE: If asked to add a door to an existing house model, you MUST use:
    "target_parent": "Workspace/House"

    IF YOU ARE ASKED TO MODIFY SOMETHING, SET CORRECT target_parent BASED ON REQUEST.
    EXAMPLE: modify script in StarterPlayerScripts. YOU WILL SET StarterPlayerScripts AS THE target_parent.
    Use target_parent for setting the parent of outer-most instances in your json response. 
    
    You will add a Item element. This item element will have a class, this class is the type of Instance of the item.
    https://create.roblox.com/docs/reference/engine/classes/Instance 
    Each class has its own properties and can also have properties infered from other classes.
    Please correctly add the correct properties for each added item.

    PROVIDE UDIM2 AS AN ARRAY OF 4 VALUES, [xScale, xOffset, yScale, yOffset].

    EVERY INSTANCE MUST HAVE A NAME.

    NAME IS NOT A PROPERTY

    Font enum must be between 0 and 45.

    Do not assign a Primary Part to a Model.
    
    BrickColor must be a number and not 0.

    Things like doors, windows, and other objects that should be open, should be NegationOperations instead of parts.
    Collect groups of parts together as models.

    Material is an Enum type.
    The default Plastic material has a very light texture, and the SmoothPlastic material has no texture at all.
    Some material textures like DiamondPlate and Granite have very visible textures. 
    Each material's texture reflects sunlight differently, especially Foil. 
    The Glass material changes rendering behavior on moderate graphics settings. 
    It applies a bit of reflectiveness.

    Name: Plastic Value:256
    Name: SmoothPlastic Value:272
    Name: Neon Value:288
    Name: Wood Value:512
    Name: WoodPlanks Value:528
    Name: Marble Value:784
    Name: Basalt Value:788
    Name: Slate Value:800
    Name: CrackedLava Value:804
    Name: Concrete Value:816
    Name: Limestone Value:820
    Name: Granite Value:832
    Name: Pavement Value:836
    Name: Brick Value:848
    Name: Pebble Value:864
    Name: Cobblestone Value:880
    Name: Rock Value:896
    Name: Sandstone Value:912
    Name: CorrodedMetal Value:1040
    Name: DiamondPlate Value:1056
    Name: Foil Value:1072
    Name: Metal Value:1088
    Name: Grass Value:1280
    Name: LeafyGrass Value:1284
    Name: Sand Value:1296
    Name: Fabric Value:1312
    Name: Snow Value:1328
    Name: Mud Value:1344
    Name: Ground Value:1360
    Name: Asphalt Value:1376
    Name: Salt Value:1392
    Name: Ice Value:1536
    Name: Glacier Value:1552
    Name: Glass Value:1568
    Name: ForceField Value:1584
    Name: Air Value:1792
    Name: Water Value:2048
    Name: Cardboard Value:2304
    Name: Carpet Value:2305
    Name: CeramicTiles Value:2306
    Name: ClayRoofTiles Value:2307
    Name: RoofShingles Value:2308
    Name: Leather Value:2309
    Name: Plaster Value:2310
    Name: Rubber Value:2311
    "#.to_string()
}