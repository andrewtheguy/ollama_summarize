//use std::collections::HashMap;
use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read};
use std::env;
use std::fs;
use std::process;


async fn summarize_content(content: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Construct the JSON body using serde_json's `json!` macro
    let json_body = json!({
        "model": "phi3:medium-128k",
        //"system": "整理一下這節目錄音文本的重點",
        "prompt": format!("整理一下這節目錄音文本的重點:\n\n{}", content),
        "options": {
          "num_ctx": 32768
        },
        "stream": false
      });

    //eprintln!("first 100 chars of content: {}", &content[..100]);
    eprintln!("Sending request to the API...");

    let client = reqwest::Client::new();

    // Send a POST request with the JSON body
    let response = client.post("http://localhost:11434/api/generate")
        .json(&json_body) // Send the JSON body
        .send()
        .await?; // Await the response

    // Parse the response body as JSON
    let response_json: Value = response.json().await?;

    let mut done_properly = false;

    if let Some(json_object) = response_json.as_object() {
        if let Some(done) = json_object.get("done"){
            if done.as_bool().unwrap() {
                if let Some(done_reason_val) = json_object.get("done_reason"){
                    let done_reason = done_reason_val.as_str().unwrap();
                    if done_reason == "stop" {
                        done_properly = true;
                        return Ok(json_object.get("response").unwrap().as_str().unwrap().to_string());
                    }else{
                        eprintln!("Done reason is not stop, might not have finished properly");
                    }
                }
            }
        }else{
            eprintln!("No done field");
        }
    }


    // Print the response JSON object
    eprintln!("Response JSON: {}", serde_json::to_string_pretty(&response_json)?);

    return Err("Failed to summarize".into());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect the command-line arguments
    let args: Vec<String> = env::args().collect();
    
    // Check if a filename was provided as an argument
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    // Get the filename from the first argument
    let filename = &args[1];

    // Read the file to a string and handle potential errors
    match fs::read_to_string(filename) {
        Ok(contents) => {
            let response = summarize_content(&contents).await?;
            println!("{}",response);
        },
        Err(err) => {
            eprintln!("Error reading file {}: {}", filename, err);
            process::exit(1);
        }
    }


    Ok(())
}