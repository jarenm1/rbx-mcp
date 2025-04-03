# Roblox MCP
MCP for roblox studio.
Gemini support only, kinda hardcoded 2.0 flash model.

# Usage
```
clone and cargo run -- --file <FILE> --api-key <KEY> --prompt <PROMPT> --context <FILE>
```
Context is optional. 
Api key can be provided as argument or via env variable.

## Context
Put anything extra you want to send to Gemini here.

# Args

-f, --file <FILE>    Input file path
 
-k, --api-key <KEY>  Gemini API key (can also be provided via GEMINI_API_KEY env variable)
 
-p, --prompt <PROMPT>  Custom prompt to send to Gemini along with the place data

-c, --context <FILE>  Context file path (markdown .md)

# Example

```
cargo run -- --file ./path/to/your/file.rbxlx --api-key <YOUR_API_KEY> --prompt "Make a cool house" --context ./path/to/your/context.md

```

