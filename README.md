# Roblox MCP

mcp for roblox studio using gemini 
hardcoded 2.0 flash model
 
# Usage
clone and cargo run -- --file <FILE> --api-key <KEY> --prompt <PROMPT>
# Args

-f, --file <FILE>    Input file path
 
-k, --api-key <KEY>  Gemini API key (can also be provided via GEMINI_API_KEY env variable)
 
-p, --prompt <PROMPT>  Custom prompt to send to Gemini along with the place data

# Example

```
cargo run -- --file ./path/to/your/file.rbxlx --api-key <YOUR_API_KEY> --prompt "Make a cool house"

```