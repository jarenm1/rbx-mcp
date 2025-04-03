# Roblox MCP
MCP for roblox studio.
Gemini support only, kinda hardcoded 2.0 flash model.

# Usage
Clone and use cargo run with file path.

Context is optional. 
Api key can be provided as argument or via env variable.

Note: Theres a live reload feature that currently does not work. Also depends on a plugin. Will rewrite it later.

## Context
Put anything extra you want to send to Gemini here.

# Args

-f, --file <FILE>    Input file path
 
-k, --api-key <KEY>  Gemini API key (can also be provided via GEMINI_API_KEY env variable)

-c, --context <FILE>  Context file path (markdown .md)

# Example

```
cargo run -- --file ./path/to/your/file.rbxlx --api-key <YOUR_API_KEY> --context ./path/to/your/context.md

```

## Example Prompt
Make me a large brick house that is 20 units tall, the walls should be red and made of brick material. Make a door that a player can walk through. Include a sloped roof that is colored black. This will require dozens of parts. Sloped roof will required orientation and correct positioning. Include pillars on the corners to improve looks of the house. Include a proper door

### Output
![image](https://github.com/user-attachments/assets/0d2f9a80-9194-4cfa-bfb0-dd1957e7072d)
