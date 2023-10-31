import json
from collections import defaultdict
from typing import List, Dict

# Module for token counting.
import utils.token_counter as token_counter

# Module for cost estimation.
import utils.cost_estimator as cost_estimator

# Constants to estimate the costs of fine-tuning.
MAX_TOKENS_PER_EXAMPLE = 4096

TARGET_EPOCHS = 3
MIN_TARGET_EXAMPLES = 100
MAX_TARGET_EXAMPLES = 25000
MIN_DEFAULT_EPOCHS = 1
MAX_DEFAULT_EPOCHS = 25


def load_dataset(data_path: str) -> List[Dict]:
    """This function loads a dataset as a list of dictionaries.
    The function returns the loaded dataset."""

    # Load the dataset.
    with open(data_path, 'r', encoding='utf-8') as f:
        dataset = [json.loads(line) for line in f]

    return dataset


def check_errors(dataset: List[Dict]) -> bool:
    """This function checks if the dataset is in a valid format.
    The function returns true, if the dataset is valid, false - otherwise."""

    # Format error checks
    format_errors = defaultdict(int)

    # Traverse each dictionary with an example.
    for ex in dataset:
        # Check if the example is a dictionary.
        if not isinstance(ex, dict):
            format_errors["data_type"] += 1
            continue

        # Check if the example contains the field "messages".
        messages = ex.get("messages", None)
        if not messages:
            format_errors["missing_messages_list"] += 1
            continue

        # Traverse each message in a dialog.
        for message in messages:
            # Check if the message is assigned a role.
            if "role" not in message or "content" not in message:
                format_errors["message_missing_key"] += 1

            # Check if there is an invalid dictionary key.
            if any(k not in ("role", "content", "name", "function_call") for k in message):
                format_errors["message_unrecognized_key"] += 1

            # Check if the message is assigned an existing role.
            if message.get("role", None) not in ("system", "user", "assistant", "function"):
                format_errors["unrecognized_role"] += 1

            content = message.get("content", None)
            function_call = message.get("function_call", None)

            if (not content and not function_call) or not isinstance(content, str):
                format_errors["missing_content"] += 1

        # Check if a dialog contains at least one assistant (ChatGPT) response.
        if not any(message.get("role", None) == "assistant" for message in messages):
            format_errors["example_missing_assistant_message"] += 1

    # If any errors found.
    if format_errors:
        print("Found errors:")
        for k, v in format_errors.items():
            print(f"{k}: {v}")
        return False

    # If no errors found.
    print("No errors found")


# Path to a dataset.
data_path = "datasets/dataset1.jsonl"

# Load dataset.
dataset = load_dataset(data_path)

# Initial dataset stats.
print("Num examples:", len(dataset))
print("First example:")
for message in dataset[0]["messages"]:
    print(message)
print()

# Check if the dataset is valid.
if check_errors(dataset):
    # There are some errors in a dataset.
    exit(0)
print()

# Display some information about estimated token consumption,
# while fine-tuning.
convo_lens = token_counter.print_token_count_info(dataset)
print()

# Display the information about approximate costs of fine-tuning a model on
# a particular dataset.
cost_estimator.estimate_costs(dataset, convo_lens, TARGET_EPOCHS, MAX_DEFAULT_EPOCHS,
                              MAX_DEFAULT_EPOCHS, MAX_DEFAULT_EPOCHS, MIN_DEFAULT_EPOCHS, MAX_TOKENS_PER_EXAMPLE)
