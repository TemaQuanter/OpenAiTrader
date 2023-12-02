use std::{
    fs,
    io::{BufRead, BufReader, Write},
};

use axum::{
    http::{HeaderName, Method},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

/// This struct is used to parse a message in a sample dialog
/// with ChatGPT.
#[derive(Deserialize, Serialize, Debug)]
struct Message {
    role: String,
    content: String,
} // end struct Message

/// This structure is used to parse the sample dialog.
#[derive(Deserialize, Serialize, Debug)]
struct SampleDialog {
    messages: Vec<Message>,
} // end struct SampleDialog

/// This function saves all changes on HTML page to the training dataset.
async fn save_changes(Json(dataset): Json<serde_json::Value>) {
    // Get all requests and responses as an array of strings.
    let mut requests: Vec<String> = Vec::new();
    let mut responses: Vec<String> = Vec::new();

    // This array stores all updated examples for a dataset.
    let mut new_examples: Vec<String> = Vec::new();

    // Traverse each request and save it as a string.
    for request in dataset["requests"].as_array().unwrap().into_iter() {
        // Unwrap the value as a string.
        let request = if let Value::String(val) = request {
            val.to_string()
        } else {
            String::new()
        }; // end if

        // Add the formatted request to the array of requests.
        requests.push(request);
    } // end for

    // Traverse each response and save it as a string.
    for response in dataset["responses"].as_array().unwrap().into_iter() {
        // Unwrap the value as a string.
        let response = if let Value::String(val) = response {
            val.to_string()
        } else {
            String::new()
        }; // end if

        // Add the formatted response to the array of responses.
        responses.push(response);
    } // end for

    // Open a file with a dataset.
    let dataset_file = fs::File::open("../datasets/dataset.jsonl").unwrap();

    // Create a buffered reader for the file
    let reader = BufReader::new(dataset_file);

    // Read the file line by line.
    for (ind, line) in reader.lines().enumerate() {
        // Unwrap a line.
        let line = line.unwrap();

        // Parse the line into a sample dialog.
        let mut sample_dialog = serde_json::from_str::<SampleDialog>(&line).unwrap();

        // Replace the last request of a user with the new request.
        sample_dialog.messages[2].content = requests[ind].to_string();

        // Replace the last reply of assistant with the response.
        sample_dialog.messages[3].content = responses[ind].to_string();

        // println!("{:#?}", sample_dialog);
        // println!("{}", responses[ind].to_string());

        // Parse sample dialog to string.
        let sample_dialog = serde_json::to_string(&sample_dialog).unwrap();

        // Save an updated example.
        new_examples.push(sample_dialog);
    } // end for

    // Open a dataset file and rewrite it.
    let mut dataset_file = fs::File::create("../datasets/dataset.jsonl").unwrap();

    // Save updated examples to the dataset file.
    for line in new_examples {
        writeln!(dataset_file, "{}", line).unwrap();
    } // end for
} // end fn save_changes

/// This function restores the data for the website.
/// It retrieves the most recent data and sends it to font-end.
async fn get_data() -> Json<serde_json::Value> {
    // Initialize an array where to store all responses.
    let mut requests: Vec<String> = Vec::new();
    let mut responses: Vec<String> = Vec::new();

    // Open a file with a dataset.
    let dataset_file = fs::File::open("../datasets/dataset.jsonl").unwrap();

    // Create a buffered reader for the file
    let reader = BufReader::new(dataset_file);

    // Read the file line by line.
    for line in reader.lines() {
        // Unwrap a line.
        let line = line.unwrap();

        // Parse the line into a sample dialog.
        let sample_dialog = serde_json::from_str::<SampleDialog>(&line).unwrap();

        requests.push(sample_dialog.messages[2].content.to_string());
        responses.push(sample_dialog.messages[3].content.to_string());
        // Add a sample response into the array with responses.
    } // end for

    // Assign the array with responses to the endpoint response.
    let response_data: serde_json::Value = json!({
        "requests": requests,
        "responses": responses
    });

    Json(response_data)
} // end fn get_data

#[tokio::main]
async fn main() {
    // Create an application router.
    let app = Router::new()
        .route("/save_changes", post(save_changes))
        .route("/get_data", get(get_data))
        .layer(
            CorsLayer::new()
                // Allow requests from any origin.
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([HeaderName::from_static("content-type")]),
        );

    // Run the server.
    axum::Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
} // end fn main
