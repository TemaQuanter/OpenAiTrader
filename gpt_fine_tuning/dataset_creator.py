# This file creates a dataset from the raw exported telegram data.

import json
from typing import List, Dict


def init_new_example(system_prompt: str) -> Dict:
    """This function returns an initialized conversation with ChatGPT."""

    # Initialize a conversation.
    conversation = dict()

    # Initialize a conversation with a system prompt and some
    # necessary data.
    conversation["messages"] = list()

    system_prompt_gpt = {"role": "system", "content": system_prompt}
    assistant_confirmation = {"role": "assistant", "content": "OK"}

    conversation["messages"].append(system_prompt_gpt)
    conversation["messages"].append(assistant_confirmation)

    return conversation


def add_example(resulting_dataset: List[Dict], system_prompt: str, user_request: str, assistant_reply: str) -> None:
    """This function creates a new example for training data."""

    # Initialize a new example.
    new_example = init_new_example(system_prompt)

    # Adapt user request and assistant reply to
    # ChatGPT training data requirements.

    user_request = {"role": "user", "content": user_request}
    assistant_reply = {"role": "assistant", "content": assistant_reply}

    # Add a user request and an assistant reply.
    new_example["messages"].append(user_request)
    new_example["messages"].append(assistant_reply)

    # Add the new example into the dataset.
    resulting_dataset.append(new_example)


def load_system_prompt(path) -> str:
    """This function loads a system prompt for ChatGPT."""

    # Initialize a response variable.
    system_prompt = ""

    # Read the system prompt from a file.
    with open(path, "r") as f:
        system_prompt = "".join(f.readlines())

    return system_prompt


def sort_comp(message) -> int:
    """This function is a custom comparison rule for an array of messages."""
    return message["id"]


# Path to raw dataset.
RAW_DATA_PATH = "raw_datasets/dataset.json"

# Initialize a raw dataset.
raw_dataset = None

# Initialize a resulting dataset.
resulting_dataset = list(dict())

# Load the file.
with open(RAW_DATA_PATH, "rb") as f:
    raw_dataset = json.load(f)

print("File is loaded")

# Sort the messages by ID.
raw_dataset = sorted(raw_dataset["messages"], key=sort_comp)

# Create arrays to store user requests and assistant replies.
user_requests = []
assistant_replies = []

# Initialize a loop counter.
i = 0

message_counter = 2

# Import a system prompt for ChatGPT.
system_prompt = load_system_prompt("../data/chatgpt_seed_prompt.txt")


# Print the first 5 messages.
while i < len(raw_dataset):
    # The message itself.
    message = raw_dataset[i]["text"]

    # if type(message) == str:
    #     if message.find("Logged in") == -1 and message.find("updated") == -1:
    #         output_file.write(f"{message}\n")
    #         output_file.write("\n" + ("-" * 25) +
    #                           str(message_counter) + ("-" * 25) + "\n")
    #         message_counter += 1

    # i += 1

    # continue

    # Skip the messages that are not plain text.
    if type(message) == str:
        # If substring is found
        if (message.find("take_profit_lower_bound") != -1) or message == "None":
            # The message is sent by an assistant.
            assistant_replies.append(message)

            # Equalize the number or assistant replies and user requests.
            while len(assistant_replies) > len(user_requests):
                user_requests.append("None")
        # If substring is found.
        elif message.find("Msg #") != -1:
            # Equalize the number or assistant replies and user requests.
            while len(user_requests) > len(assistant_replies):
                assistant_replies.append("None")

            # The message is sent by a user.
            user_requests.append(message)
    i += 1

# Generate examples based on user requests and assistant replies.
for i in range(min(len(user_requests), len(assistant_replies))):
    # Generate an example.
    add_example(resulting_dataset, system_prompt,
                user_requests[i], assistant_replies[i])

# Save the training data to a file.
with open("datasets/dataset_test.jsonl", "w") as f:
    # Write each example on a separate line.
    for ex in resulting_dataset:
        f.write(json.dumps(ex))
        f.write("\n")


# Save the training data to html file for convenient data formatting.
with open("html/dataset.html", "w") as f:
    # Initialize boilerplate code.
    boilerplate_code = None

    # Import a boilerplate code.
    with open("html/boilerplate.html") as boilerplate_f:
        boilerplate_code = boilerplate_f.readlines()

    # Write each example in a formatted way.
    f.writelines(boilerplate_code[:-4])
    for i, ex in enumerate(resulting_dataset):
        f.write(f"<p class=\"block_title\">Example #{i + 1}</p>\n")
        f.write("<div class=\"block\">\n")
        # f.write(
        #     "<form action=\"http://0.0.0.0:9090/save_changes\" method=\"POST\" onsubmit=\"prepareFormSubmission()\">\n")
        f.write("<p class=\"request\">Request</p>\n")
        f.write(
            f"<pre class=\"user\">{ex['messages'][2]['content']}</pre>\n")
        f.write("<p class=\"response\">Response</p>\n")
        f.write(
            f"<pre class=\"assistant\" contenteditable=\"true\">{ex['messages'][3]['content']}</pre>\n")
        f.write(
            f"<input type=\"hidden\" class=\"hiddenInput\" name=\"response_{i + 1}\">\n")
        f.write(
            "<button type=\"submit\">Save All changes</button>")
        # f.write("</form>\n")
        f.write("</div>\n")
    f.writelines(boilerplate_code[-4:])
