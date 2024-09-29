use serde::Deserialize;
//use std::collections::HashMap;
use serde::Serialize;
use serde_json::{Value};
use std::env;
use std::fs;
use std::process;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures::stream::TryStreamExt;

// {"model":"mistral:7b-instruct-v0.3-q8_0","created_at":"2024-09-29T00:53:57.27188Z","response":"","done":true,"done_reason":"stop","context":[3,29473,5083,1117,1040,7980,5813,29572,29473,4,29473,1183,7980,8813,5813,3708,1066,1032,2527,2755,9720,1059,1724,22403,29491,2452,23718,29493,1458,1117,2037,1350,1070,2349,10072,29493,19478,9367,29510,29481,14557,29493,1146,1559,10432,1163,12928,22417,1072,9655,14706,1065,1040,3191,2027,1158,23060,26270,1072,21826,29491,9604,2829,1427,20135,1043,27563,29481,1072,5507,21763,1448,5829,1589,1567,10072,1505,3528,1210,10452,29491,1904,23718,1520,1038,2300,1065,1312,15046,29493,1448,5813,2829,1117,21763,5851,1581,3050,1245,1040,2138,2304,7980,29493,3260,1146,5073,5813,29491],"total_duration":18202306917,"load_duration":15613875834,"prompt_eval_count":11,"prompt_eval_duration":90585000,"eval_count":99,"eval_duration":2496110000}
#[derive(Debug, Clone, Deserialize)]
struct MyTestStructure {
    response: String,
    done: bool,
    done_reason: Option<String>,
}

#[derive(Serialize)]
struct Options {
    num_ctx: u32,
    temperature: f32,
    num_predict: i32,
    //top_p: f32, // Uncomment if you want to include "top_p" as an optional field
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    //system: String, // Uncomment if you want to include "system" as an optional field
    prompt: Option<String>,
    options: Options,
    stream: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            num_ctx: 32768,
            temperature: 0.0,
            num_predict: -1,
            //top_p: 0.8, // Uncomment if you want to include "top_p" as an optional field
        }
    }
    
}

impl Default for RequestBody {
    fn default() -> Self {
        RequestBody {
            model: "phi3:medium-128k".to_string(),
            //system: "整理一下這節目錄音文本的重點".to_string(), // Uncomment if needed
            prompt: None,
            options: Options::default(),
            stream: false,
        }
    }
    
}


async fn summarize_content(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let request_body = RequestBody{ stream: false,
        prompt: Some(format!("整理一下這節目錄音文本的重點:\n\n{}", content)),
         ..RequestBody::default() };
    //eprintln!("first 100 chars of content: {}", &content[..100]);
    eprintln!("Sending request to the API...");

    let client = reqwest::Client::new();

    // Send a POST request with the JSON body
    let response = client.post("http://localhost:11434/api/generate")
        .json(&request_body) // Send the JSON body
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
                        println!("{}",json_object.get("response").unwrap().as_str().unwrap().to_string());
                        return Ok(());
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
    eprintln!("Response JSON for debug: {}", serde_json::to_string_pretty(&response_json)?);

    return Err("Failed to summarize".into());
}


async fn summarize_content_with_streaming(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let request_body = RequestBody{ stream: true,
        prompt: Some(format!("整理一下這節目錄音文本的重點:\n\n{}", content)),
         ..RequestBody::default() };

    //eprintln!("first 100 chars of content: {}", &content[..100]);
    eprintln!("Sending request to the API...");

    let client = reqwest::Client::new();

    // Send a POST request with the JSON body
    let response = client.post("http://localhost:11434/api/generate")
        .json(&request_body) // Send the JSON body
        .send()
        .await?; // Await the response
 

    // // Make sure the response is successful
    // if !response.status().is_success() {
    //     eprintln!("Error: Failed to fetch the streaming data");
    //     return Ok(());
    // }

    let stream = response.bytes_stream().map_err(std::io::Error::other);
    let stream_reader = tokio_util::io::StreamReader::new(stream);

    // Create a buffered reader to read line by line
    let mut reader = BufReader::new(stream_reader).lines();

    // Process each line
    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue; // Skip empty lines
        }

        // Parse the line as a JSON object
        match serde_json::from_str::<MyTestStructure>(&line) {
            Ok(result_line) => {
                print!("{}",result_line.response);
                if result_line.done {
                    if let Some(done_reason) = result_line.done_reason {
                        if done_reason != "stop" {
                            return Err("done with a reason other than stop".into());
                        }else{
                            println!();
                            return Ok(());
                        }
                    }
                    //break;
                }
            }
            Err(err) => {
                eprintln!("Error parsing JSON: {:?}", err);
            }
        }
    }

    return Err("Failed to summarize".into());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect the command-line arguments
    let args: Vec<String> = env::args().collect();
    let stream = true;
    
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
            if(stream){
                summarize_content_with_streaming(&contents).await?;
            }else{
                summarize_content(&contents).await?;
            }
        },
        Err(err) => {
            eprintln!("Error reading file {}: {}", filename, err);
            process::exit(1);
        }
    }


    Ok(())
}