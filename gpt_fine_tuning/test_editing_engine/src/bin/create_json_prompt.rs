// This file is needed to create a new system prompt for already prepared ChatGPT
// training dataset.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
};

use serde::{Deserialize, Serialize};

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

fn main() {
    // This array stores all updated examples for a dataset.
    let mut new_examples: Vec<String> = Vec::new();

    // Get a file with an updated system prompt.
    let system_prompt = fs::read_to_string("../../data/chatgpt_seed_prompt_spot.txt").unwrap();

    // Open a file with a dataset.
    let dataset_file = fs::File::open("../datasets/dataset.jsonl").unwrap();

    // Create a buffered reader for the file
    let reader = BufReader::new(dataset_file);

    // Read the file line by line.
    for line in reader.lines() {
        // Unwrap a line.
        let line = line.unwrap();

        // Parse the line into a sample dialog.
        let mut sample_dialog = serde_json::from_str::<SampleDialog>(&line).unwrap();

        // Replace the last reply of assistant with the response.
        sample_dialog.messages[0].content = system_prompt.to_string();

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
} // end fn main
